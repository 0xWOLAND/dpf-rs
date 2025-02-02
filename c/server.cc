#include "server.h"

#include <memory>
#include <string>
#include <vector>

#include "absl/functional/any_invocable.h"
#include "absl/status/status.h"
#include "absl/strings/string_view.h"
#include "external/google_dpf/dpf/distributed_point_function.h"
#include "external/google_dpf/pir/dense_dpf_pir_database.h"
#include "external/google_dpf/pir/dense_dpf_pir_server.h"
#include "external/google_dpf/pir/private_information_retrieval.pb.h"

using distributed_point_functions::DenseDpfPirServer;
using distributed_point_functions::PirConfig;
using distributed_point_functions::PirRequest;
using distributed_point_functions::PirResponse;

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

struct DpfPirServer_st {
    std::unique_ptr<DenseDpfPirServer> impl;
    // Store callbacks and user data for leader/helper
    DpfPirForwardHelperRequestFn forward_fn;
    DpfPirDecryptHelperRequestFn decrypt_fn;
    void* user_data;
};

extern "C" {

DpfPirStatus dpf_pir_server_create_leader(
    const DpfPirConfig* config,
    void* database,
    DpfPirForwardHelperRequestFn forward_fn,
    void* user_data,
    DpfPirServer* server) {
    
    if (!config || !database || !forward_fn || !server) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    PirConfig pir_config;
    auto* dense_config = pir_config.mutable_dense_dpf_pir_config();
    dense_config->set_num_elements(config->num_elements);

    // Create forward function wrapper
    auto sender = [forward_fn, user_data](
        const PirRequest& request,
        absl::AnyInvocable<void()> while_waiting) -> absl::StatusOr<PirResponse> {
        
        // Serialize request
        DpfPirBuffer helper_request;
        std::string serialized = request.SerializeAsString();
        if (!allocate_buffer(&helper_request, serialized)) {
            return absl::ResourceExhaustedError("Failed to allocate helper request buffer");
        }

        // Call user's forward function
        DpfPirBuffer response_buffer = {nullptr, 0};
        DpfPirStatus status = forward_fn(&helper_request, &response_buffer, user_data);
        dpf_pir_buffer_free(&helper_request);

        if (status != DPF_PIR_OK) {
            dpf_pir_buffer_free(&response_buffer);
            return absl::InternalError("Forward function failed");
        }

        // Parse response
        PirResponse response;
        if (!response.ParseFromArray(response_buffer.data, response_buffer.size)) {
            dpf_pir_buffer_free(&response_buffer);
            return absl::InternalError("Failed to parse helper response");
        }
        
        dpf_pir_buffer_free(&response_buffer);
        return response;
    };

    // Get database pointer (need to cast from void*)
    auto* db_ptr = static_cast<DenseDpfPirServer::Database*>(database);

    auto result = DenseDpfPirServer::CreateLeader(
        pir_config, std::unique_ptr<DenseDpfPirServer::Database>(db_ptr), 
        std::move(sender));

    if (!result.ok()) {
        return convert_status(result.status());
    }

    *server = new DpfPirServer_st{
        std::move(result.value()),
        forward_fn,
        nullptr,
        user_data
    };
    
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_server_create_helper(
    const DpfPirConfig* config,
    void* database,
    DpfPirDecryptHelperRequestFn decrypt_fn,
    void* user_data,
    DpfPirServer* server) {
    
    if (!config || !database || !decrypt_fn || !server) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    PirConfig pir_config;
    auto* dense_config = pir_config.mutable_dense_dpf_pir_config();
    dense_config->set_num_elements(config->num_elements);

    // Create decrypt function wrapper
    auto decrypter = [decrypt_fn, user_data](
        absl::string_view ciphertext,
        absl::string_view context_info) -> absl::StatusOr<std::string> {
        
        DpfPirBuffer cipher_buffer = {
            const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(ciphertext.data())),
            ciphertext.size()
        };
        
        DpfPirBuffer plain_buffer = {nullptr, 0};
        DpfPirStatus status = decrypt_fn(
            &cipher_buffer, 
            context_info.data(),
            &plain_buffer,
            user_data);

        if (status != DPF_PIR_OK) {
            return absl::InternalError("Decrypt function failed");
        }

        std::string result(reinterpret_cast<char*>(plain_buffer.data), 
                         plain_buffer.size);
        dpf_pir_buffer_free(&plain_buffer);
        return result;
    };

    // Get database pointer
    auto* db_ptr = static_cast<DenseDpfPirServer::Database*>(database);

    auto result = DenseDpfPirServer::CreateHelper(
        pir_config, 
        std::unique_ptr<DenseDpfPirServer::Database>(db_ptr),
        std::move(decrypter));

    if (!result.ok()) {
        return convert_status(result.status());
    }

    *server = new DpfPirServer_st{
        std::move(result.value()),
        nullptr,
        decrypt_fn,
        user_data
    };
    
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_server_create_plain(
    const DpfPirConfig* config,
    void* database,
    DpfPirServer* server) {
    
    if (!config || !database || !server) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    PirConfig pir_config;
    auto* dense_config = pir_config.mutable_dense_dpf_pir_config();
    dense_config->set_num_elements(config->num_elements);

    // Get database pointer
    auto* db_ptr = static_cast<DenseDpfPirServer::Database*>(database);

    auto result = DenseDpfPirServer::CreatePlain(
        pir_config,
        std::unique_ptr<DenseDpfPirServer::Database>(db_ptr));

    if (!result.ok()) {
        return convert_status(result.status());
    }

    *server = new DpfPirServer_st{
        std::move(result.value()),
        nullptr,
        nullptr,
        nullptr
    };
    
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_server_handle_request(
    DpfPirServer server,
    const DpfPirRequest* request,
    DpfPirResponse* response) {
    
    if (!server || !request || !response) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    // Parse the request
    PirRequest pir_request;
    if (!pir_request.ParseFromArray(request->leader_request.data, 
                                  request->leader_request.size)) {
        set_last_error("Failed to parse request");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    // Handle the request
    auto result = server->impl->HandleRequest(pir_request);
    if (!result.ok()) {
        return convert_status(result.status());
    }

    // Serialize the response
    std::string serialized = result.value().SerializeAsString();
    if (!allocate_buffer(&response->response, serialized)) {
        return DPF_PIR_OUT_OF_MEMORY;
    }

    return DPF_PIR_OK;
}

void dpf_pir_server_destroy(DpfPirServer server) {
    delete server;
}

void dpf_pir_buffer_free(DpfPirBuffer* buffer) {
    if (buffer) {
        free(buffer->data);
        buffer->data = nullptr;
        buffer->size = 0;
    }
}

void dpf_pir_request_free(DpfPirRequest* request) {
    if (request) {
        dpf_pir_buffer_free(&request->leader_request);
        dpf_pir_buffer_free(&request->helper_request);
    }
}

void dpf_pir_response_free(DpfPirResponse* response) {
    if (response) {
        dpf_pir_buffer_free(&response->response);
    }
}

const char* dpf_pir_get_last_error() {
    return g_last_error.c_str();
}

} // extern "C"