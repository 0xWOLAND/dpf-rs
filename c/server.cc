#include "server.h"

#include <memory>
#include <string>
#include <vector>
#include <mutex>

#include "absl/status/statusor.h"
#include "absl/functional/any_invocable.h"
#include "external/google_dpf/pir/dense_dpf_pir_server.h"
#include "external/google_dpf/pir/dense_dpf_pir_database.h"

using distributed_point_functions::DenseDpfPirServer;
using distributed_point_functions::PirConfig;
using distributed_point_functions::PirRequest;
using distributed_point_functions::PirResponse;
using distributed_point_functions::DenseDpfPirDatabase;

// Complete database structure definition
struct DpfPirDatabase_st {
    std::unique_ptr<DenseDpfPirDatabase::Interface> impl;  // We own this now
    size_t size;

    static absl::StatusOr<std::unique_ptr<DpfPirDatabase_st>> Create(
            const std::vector<std::string>& values,
            DenseDpfPirDatabase::Builder* builder = nullptr) {
        std::unique_ptr<DenseDpfPirDatabase::Builder> owned_builder;
        if (builder == nullptr) {
            owned_builder = std::make_unique<DenseDpfPirDatabase::Builder>();
            builder = owned_builder.get();
        }

        for (const auto& value : values) {
            builder->Insert(value);
        }

        auto build_result = builder->Build();
        if (!build_result.ok()) {
            return build_result.status();
        }

        auto db = std::make_unique<DpfPirDatabase_st>();
        db->impl = std::move(build_result).value();
        db->size = values.size();
        return db;
    }
};

namespace {

// Thread-local error message storage with mutex protection
struct ErrorStorage {
    std::string message;
    std::mutex mutex;
};
thread_local ErrorStorage g_error_storage;

void set_last_error(const std::string& error) {
    std::lock_guard<std::mutex> lock(g_error_storage.mutex);
    g_error_storage.message = error;
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

struct DpfPirServer_st {
    std::unique_ptr<DenseDpfPirServer> impl;
    DpfPirServerConfig config;  // Copy of the config
};

DpfPirStatus dpf_pir_server_create(
    const DpfPirServerConfig* config,
    DpfPirServer* server) {
    if (!config || !server || !config->database) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    absl::StatusOr<std::unique_ptr<DenseDpfPirServer>> result;
    if (config->role == DPF_PIR_SERVER_LEADER) {
        // result = DenseDpfPirServer::CreateLeader(
        //     config->pir_config,
        //     std::move(config->database)
        // );
    } else {
        // TODO: Implement helper creation logic
    }

    return DPF_PIR_OK;
}

void dpf_pir_server_destroy(DpfPirServer server) {
    delete server;
}

DpfPirStatus dpf_pir_server_handle_request(
    DpfPirServer server,
    const DpfPirBuffer* request,
    DpfPirBuffer* response) {
    if (!server || !request || !response) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    PirRequest pir_request;
    if (!pir_request.ParseFromArray(request->data, request->size)) {
        set_last_error("Failed to parse request");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    auto result = server->impl->HandleRequest(pir_request);
    if (!result.ok()) {
        return convert_status(result.status());
    }

    std::string serialized = result->SerializeAsString();
    if (!allocate_buffer(response, serialized)) {
        return DPF_PIR_OUT_OF_MEMORY;
    }

    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_server_get_public_params(
    DpfPirServer server,
    DpfPirBuffer* params) {
    if (!server || !params) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    const auto& public_params = server->impl->GetPublicParams();
    std::string serialized = public_params.SerializeAsString();

    if (!allocate_buffer(params, serialized)) {
        return DPF_PIR_OUT_OF_MEMORY;
    }

    return DPF_PIR_OK;
}

const char* dpf_pir_get_last_error() {
    std::lock_guard<std::mutex> lock(g_error_storage.mutex);
    return g_error_storage.message.c_str();
}