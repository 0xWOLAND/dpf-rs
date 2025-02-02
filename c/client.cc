#include "client.h"

#include <memory>
#include <string>
#include <vector>

#include "absl/status/status.h"
#include "absl/strings/string_view.h"
#include "external/google_dpf/dpf/distributed_point_function.h"
#include "external/google_dpf/pir/dense_dpf_pir_client.h"
#include "external/google_dpf/pir/private_information_retrieval.pb.h"

using distributed_point_functions::DenseDpfPirClient;
using distributed_point_functions::PirConfig;
using distributed_point_functions::PirRequest;
using distributed_point_functions::PirResponse;
using distributed_point_functions::PirRequestClientState;

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

struct DpfPirClient_st {
    std::unique_ptr<DenseDpfPirClient> impl;
    DpfPirEncryptRequestFn encrypt_fn;
    void* user_data;
};

extern "C" {

DpfPirStatus dpf_pir_client_create(
    const DpfPirConfig* config,
    DpfPirEncryptRequestFn encrypt_fn,
    void* user_data,
    const char* encryption_context_info,
    DpfPirClient* client) {
    
    if (!config || !encrypt_fn || !client) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    PirConfig pir_config;
    auto* dense_config = pir_config.mutable_dense_dpf_pir_config();
    dense_config->set_num_elements(config->database_size);

    // Create encrypt function wrapper
    auto encrypter = [encrypt_fn, user_data](
        absl::string_view plain_pir_request,
        absl::string_view context_info) -> absl::StatusOr<std::string> {
        
        DpfPirBuffer plaintext = {
            const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(plain_pir_request.data())),
            plain_pir_request.size()
        };
        
        DpfPirBuffer ciphertext = {nullptr, 0};
        DpfPirStatus status = encrypt_fn(
            &plaintext,
            context_info.data(),
            &ciphertext,
            user_data);

        if (status != DPF_PIR_OK) {
            return absl::InternalError("Encrypt function failed");
        }

        std::string result(reinterpret_cast<char*>(ciphertext.data), 
                         ciphertext.size);
        dpf_pir_buffer_free(&ciphertext);
        return result;
    };

    auto result = DenseDpfPirClient::Create(
        pir_config,
        std::move(encrypter),
        encryption_context_info ? encryption_context_info : "");

    if (!result.ok()) {
        return convert_status(result.status());
    }

    *client = new DpfPirClient_st{
        std::move(result.value()),
        encrypt_fn,
        user_data
    };
    
    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_client_create_request(
    DpfPirClient client,
    const int32_t* indices,
    size_t num_indices,
    DpfPirRequest* request) {
    
    if (!client || !indices || !request || num_indices == 0) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    std::vector<int> cpp_indices(indices, indices + num_indices);
    
    // Create plain requests
    auto result = client->impl->CreatePlainRequests(cpp_indices);
    if (!result.ok()) {
        return convert_status(result.status());
    }

    auto& [leader_req, helper_req, client_state] = *result;
    
    // Serialize requests and state
    std::string leader_serialized = leader_req.SerializeAsString();
    std::string helper_serialized = helper_req.SerializeAsString();
    std::string state_serialized = client_state.SerializeAsString();

    // Allocate buffers
    if (!allocate_buffer(&request->leader_request, leader_serialized) ||
        !allocate_buffer(&request->helper_request, helper_serialized) ||
        !allocate_buffer(&request->client_state, state_serialized)) {
        dpf_pir_request_free(request);
        return DPF_PIR_OUT_OF_MEMORY;
    }

    return DPF_PIR_OK;
}

DpfPirStatus dpf_pir_client_handle_response(
    DpfPirClient client,
    const DpfPirBuffer* response,
    const DpfPirBuffer* client_state,
    DpfPirResponse* result) {
    
    if (!client || !response || !client_state || !result) {
        set_last_error("Invalid arguments");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    // Parse response and state
    PirResponse pir_response;
    PirRequestClientState state;
    
    if (!pir_response.ParseFromArray(response->data, response->size) ||
        !state.ParseFromArray(client_state->data, client_state->size)) {
        set_last_error("Failed to parse response or state");
        return DPF_PIR_INVALID_ARGUMENT;
    }

    // Handle response
    auto handle_result = client->impl->HandleResponse(pir_response, state);
    if (!handle_result.ok()) {
        return convert_status(handle_result.status());
    }

    // Convert result to C format
    const auto& values = *handle_result;
    result->num_values = values.size();
    result->values = static_cast<char**>(malloc(values.size() * sizeof(char*)));
    result->lengths = static_cast<size_t*>(malloc(values.size() * sizeof(size_t)));
    
    if (!result->values || !result->lengths) {
        dpf_pir_response_free(result);
        return DPF_PIR_OUT_OF_MEMORY;
    }

    for (size_t i = 0; i < values.size(); ++i) {
        result->lengths[i] = values[i].size();
        result->values[i] = static_cast<char*>(malloc(values[i].size()));
        if (!result->values[i]) {
            dpf_pir_response_free(result);
            return DPF_PIR_OUT_OF_MEMORY;
        }
        memcpy(result->values[i], values[i].data(), values[i].size());
    }

    return DPF_PIR_OK;
}

void dpf_pir_client_destroy(DpfPirClient client) {
    delete client;
}

void dpf_pir_request_free(DpfPirRequest* request) {
    if (request) {
        dpf_pir_buffer_free(&request->leader_request);
        dpf_pir_buffer_free(&request->helper_request);
        dpf_pir_buffer_free(&request->client_state);
    }
}

void dpf_pir_response_free(DpfPirResponse* response) {
    if (response) {
        if (response->values) {
            for (size_t i = 0; i < response->num_values; ++i) {
                free(response->values[i]);
            }
            free(response->values);
        }
        free(response->lengths);
        response->values = nullptr;
        response->lengths = nullptr;
        response->num_values = 0;
    }
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