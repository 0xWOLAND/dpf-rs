#include "database.h"

#include <memory>
#include <string>
#include <vector>

#include "absl/status/status.h"
#include "absl/status/statusor.h"
#include "absl/types/span.h"
#include "dpf/xor_wrapper.h"
#include "external/google_dpf/pir/dense_dpf_pir_database.h"

using distributed_point_functions::DenseDpfPirDatabase;
using distributed_point_functions::XorWrapper;
using BlockType = XorWrapper<absl::uint128>;

namespace {

// Thread-local error message storage
thread_local std::string g_last_error;

void set_last_error(const std::string& error) {
    g_last_error = error;
}

// Helper to convert status to DpfPirStatus
DpfPirStatus convert_status(const absl::Status& status) {
    if (status.ok()) return DPF_PIR_OK;
    
    set_last_error(std::string(status.message()));
    
    switch (status.code()) {
        case absl::StatusCode::kInvalidArgument:
            return DPF_PIR_INVALID_ARGUMENT;
        case absl::StatusCode::kFailedPrecondition:
            return DPF_PIR_FAILED_PRECONDITION;
        case absl::StatusCode::kResourceExhausted:
            return DPF_PIR_OUT_OF_MEMORY;
        default:
            return DPF_PIR_INTERNAL_ERROR;
    }
}

// Helper to allocate and copy buffer
bool allocate_buffer(DpfPirBuffer* dst, const std::string& src) {
    dst->size = src.size();
    dst->data = static_cast<uint8_t*>(malloc(dst->size));
    if (!dst->data) {
        set_last_error("Failed to allocate memory");
        return false;
    }
    memcpy(dst->data, src.data(), dst->size);
    return true;
}

} // namespace

struct DpfPirDatabase_st {
    std::unique_ptr<DenseDpfPirDatabase> impl;
};

struct DpfPirDatabaseBuilder_st {
    std::unique_ptr<DenseDpfPirDatabase::Builder> impl;
};

extern "C" {

DpfPirStatus dpf_pir_builder_create(DpfPirDatabaseBuilder* builder) {
    if (!builder) {
        set_last_error("Null builder pointer");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    *builder = new DpfPirDatabaseBuilder_st{
        std::make_unique<DenseDpfPirDatabase::Builder>()
    };
    return DPF_PIR_OK;
}

void dpf_pir_builder_destroy(DpfPirDatabaseBuilder builder) {
    delete builder;
}

DpfPirStatus dpf_pir_builder_insert(DpfPirDatabaseBuilder builder,
                                   const uint8_t* value,
                                   size_t value_length) {
    if (!builder || !value) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }
    
    builder->impl->Insert(std::string(reinterpret_cast<const char*>(value), 
                                    value_length));
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_builder_clear(DpfPirDatabaseBuilder builder) {
    if (!builder) {
        set_last_error("Null builder");
        return DPF_PIR_INVALID_ARGUMENT;
    }
    
    builder->impl->Clear();
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_builder_build(DpfPirDatabaseBuilder builder,
                                  DpfPirDatabase* database) {
    if (!builder || !database) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    auto result = builder->impl->Build();
    if (!result.ok()) {
        return convert_status(result.status());
    }

    // Cast the interface pointer to the concrete implementation
    auto* concrete_db = dynamic_cast<DenseDpfPirDatabase*>(result.value().release());
    if (!concrete_db) {
        set_last_error("Failed to cast database to concrete type");
        return DPF_PIR_INTERNAL_ERROR;
    }

    *database = new DpfPirDatabase_st{std::unique_ptr<DenseDpfPirDatabase>(concrete_db)};
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_database_size(const DpfPirDatabase database,
                                  size_t* size) {
    if (!database || !size) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }
    
    *size = database->impl->size();
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_database_selection_bits(const DpfPirDatabase database,
                                           size_t* num_bits) {
    if (!database || !num_bits) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }
    
    *num_bits = database->impl->num_selection_bits();
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_database_inner_product(const DpfPirDatabase database,
                                           const DpfPirBuffer* selections,
                                           size_t num_selections,
                                           DpfPirBuffer* result) {
    if (!database || !selections || !result || num_selections == 0) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    std::vector<std::vector<BlockType>> selection_blocks;
    selection_blocks.reserve(num_selections);

    for (size_t i = 0; i < num_selections; ++i) {
        const size_t num_blocks = (selections[i].size + sizeof(BlockType) - 1) 
                                / sizeof(BlockType);
        std::vector<BlockType> blocks(num_blocks);
        memcpy(blocks.data(), selections[i].data, selections[i].size);
        selection_blocks.push_back(std::move(blocks));
    }

    auto inner_product = database->impl->InnerProductWith(selection_blocks);
    if (!inner_product.ok()) {
        return convert_status(inner_product.status());
    }

    // Concatenate results into a single buffer
    std::string concatenated;
    for (const auto& str : *inner_product) {
        concatenated += str;
    }

    if (!allocate_buffer(result, concatenated)) {
        return DPF_PIR_OUT_OF_MEMORY;
    }

    return DPF_PIR_OK;
}

void dpf_pir_database_destroy(DpfPirDatabase database) {
    delete database;
}

void dpf_pir_buffer_free(DpfPirBuffer* buffer) {
    if (buffer) {
        free(buffer->data);
        buffer->data = nullptr;
        buffer->size = 0;
    }
}

const char* dpf_pir_get_last_error() {
    return g_last_error.c_str();
}

} // extern "C"