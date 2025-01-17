#include "server.h"

#include <memory>
#include <string>

#include "absl/status/statusor.h"
#include "absl/functional/any_invocable.h"
#include "external/google_dpf/pir/dense_dpf_pir_server.h"
#include "external/google_dpf/pir/private_information_retrieval.pb.h"
#include "external/google_dpf/pir/dense_dpf_pir_database.h"

using distributed_point_functions::DenseDpfPirServer;
using distributed_point_functions::PirConfig;
using distributed_point_functions::PirRequest;
using distributed_point_functions::PirResponse;
using distributed_point_functions::DenseDpfPirDatabase;

// Complete the database structure definition
struct DpfPirDatabase_st {
    explicit DpfPirDatabase_st(const DenseDpfPirDatabase::Interface& db_impl) 
        : size(db_impl.size()) {}
    size_t size;
};

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

// Forward declaration of the implementation
class DpfPirServerImpl : public DenseDpfPirServer {
public:
    using DenseDpfPirServer::DenseDpfPirServer;
    using DenseDpfPirServer::HandlePlainRequest;
};

struct DpfPirServer_st {
    std::unique_ptr<DpfPirServerImpl> impl;
    DpfPirServerConfig config;
};

DpfPirStatus dpf_pir_server_create(
    const DpfPirServerConfig* config,
    DpfPirServer* server) {
    if (!config || !server || !config->database) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    PirConfig pir_config;
    auto* dense_config = pir_config.mutable_dense_dpf_pir_config();
    dense_config->set_num_elements(config->database->size);

    absl::StatusOr<std::unique_ptr<DenseDpfPirServer>> result;

    if (config->role == DPF_PIR_SERVER_LEADER) {
        auto forward_fn = 
            [config](const PirRequest& request, 
                    absl::AnyInvocable<void()> done) 
            -> absl::StatusOr<PirResponse> {
            std::string serialized = request.SerializeAsString();
            DpfPirBuffer buffer = {
                reinterpret_cast<uint8_t*>(const_cast<char*>(serialized.data())),
                serialized.size()
            };
            
            DpfPirStatus status = config->callbacks.forward_fn(&buffer, 
                config->callback_user_data);
                
            if (status != DPF_PIR_OK) {
                return absl::InvalidArgumentError("Forward callback failed");
            }
            
            done();
            return PirResponse{};
        };

        auto builder = std::make_unique<DenseDpfPirDatabase::Builder>();
        // TODO: Fill the database builder with data from config->database
        
        auto database_result = builder->Build();
        if (!database_result.ok()) {
            return convert_status(database_result.status());
        }
        auto database = std::move(database_result).value();

        result = DenseDpfPirServer::CreateLeader(
            pir_config,
            std::move(database),
            std::move(forward_fn));
    } else {
        auto decrypt_fn = 
            [config](absl::string_view encrypted,
                    absl::string_view context_info) 
            -> absl::StatusOr<std::string> {
            DpfPirBuffer enc_buffer = {
                reinterpret_cast<uint8_t*>(const_cast<char*>(encrypted.data())),
                encrypted.size()
            };
            DpfPirBuffer dec_buffer = {nullptr, 0};
            
            DpfPirStatus status = config->callbacks.decrypt_fn(&enc_buffer, 
                &dec_buffer, config->callback_user_data);
            if (status != DPF_PIR_OK) {
                return absl::InvalidArgumentError("Decrypt callback failed");
            }
            
            std::string result(reinterpret_cast<char*>(dec_buffer.data), 
                dec_buffer.size);
            dpf_pir_buffer_free(&dec_buffer);
            return result;
        };

        auto builder = std::make_unique<DenseDpfPirDatabase::Builder>();
        // TODO: Fill the database builder with data from config->database
        
        auto database_result = builder->Build();
        if (!database_result.ok()) {
            return convert_status(database_result.status());
        }
        auto database = std::move(database_result).value();

        result = DenseDpfPirServer::CreateHelper(
            pir_config,
            std::move(database),
            std::move(decrypt_fn));
    }

    if (!result.ok()) {
        return convert_status(result.status());
    }

    *server = new DpfPirServer_st{
        std::unique_ptr<DpfPirServerImpl>(
            static_cast<DpfPirServerImpl*>(result.value().release())),
        *config
    };
    
    return DPF_PIR_OK;
}

void dpf_pir_server_destroy(DpfPirServer server) {
    delete server;
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

    auto result = server->impl->HandlePlainRequest(pir_request);
    if (!result.ok()) {
        return convert_status(result.status());
    }

    std::string serialized = result->SerializeAsString();
    if (!allocate_buffer(response, serialized)) {
        return DPF_PIR_OUT_OF_MEMORY;
    }

    return DPF_PIR_OK;
}

const char* dpf_pir_get_last_error() {
    return g_last_error.c_str();
}