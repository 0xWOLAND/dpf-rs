#include "server.h"
#include "external/distributed_point_functions/pir/private_information_retrieval.pb.h"
#include "external/distributed_point_functions/pir/prng/aes_128_ctr_seeded_prng.h"
#include "external/distributed_point_functions/pir/dense_dpf_pir_database.h"
#include "external/distributed_point_functions/pir/dense_dpf_pir_server.h"
#include "base64_utils.h"

#include <memory>
#include <string>
#include <vector>
#include <mutex>
#include <thread>
#include <cmath>

using namespace distributed_point_functions;

// Constants
constexpr int kBitsPerBlock = 128;

// Internal server state structure
struct ServerState {
    std::unique_ptr<DenseDpfPirServer> server;
    std::unique_ptr<DenseDpfPirServer::Database> database;
    std::unique_ptr<DistributedPointFunction> dpf;
    std::vector<std::string> elements;
    PirConfig config;
    DpfParameters params;
};

// Helper function to create database from elements
template <typename Database>
absl::StatusOr<std::unique_ptr<typename Database::Interface>>
CreateDatabase(const std::vector<typename Database::RecordType>& elements) {
    auto builder = std::make_unique<typename Database::Builder>();
    
    for (const auto& element : elements) {
        builder->Insert(element);
    }
    
    return builder->Build();
}

extern "C" {

pir_status_t pir_server_create(const char** elements, int num_elements, void** server_handle) {
    if (!elements || num_elements <= 0 || !server_handle) {
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = new ServerState();
        
        // Setup config
        state->config.mutable_dense_dpf_pir_config()->set_num_elements(num_elements);

        // Setup DPF parameters
        state->params.mutable_value_type()->mutable_xor_wrapper()->set_bitsize(kBitsPerBlock);
        state->params.set_log_domain_size(
            static_cast<int>(std::ceil(std::log2(num_elements))));

        // Create DPF instance
        auto status_or_dpf = DistributedPointFunction::Create(state->params);
        if (!status_or_dpf.ok()) {
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->dpf = std::move(status_or_dpf.value());

        // Copy elements
        state->elements.reserve(num_elements);
        for (int i = 0; i < num_elements; i++) {
            if (!elements[i]) {
                delete state;
                return PIR_ERROR_INVALID_ARGUMENT;
            }
            state->elements.push_back(elements[i]);
        }

        // Create database
        auto status_or_database = CreateDatabase<DenseDpfPirDatabase>(state->elements);
        if (!status_or_database.ok()) {
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->database = std::move(status_or_database.value());

        // Create server
        auto status_or_server = DenseDpfPirServer::CreatePlain(state->config, std::move(state->database));
        if (!status_or_server.ok()) {
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->server = std::move(status_or_server.value());

        *server_handle = state;
        return PIR_SUCCESS;

    } catch (const std::exception& e) {
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_server_process_request(void* server_handle, const char* request_base64, char** response_base64) {
    if (!server_handle || !request_base64 || !response_base64) {
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = static_cast<ServerState*>(server_handle);

        // Decode base64 request
        std::string serialized_request = base64_decode(request_base64);
        
        // Deserialize request
        PirRequest deserialized_request;
        if (!deserialized_request.ParseFromString(serialized_request)) {
            return PIR_ERROR_PROCESSING;
        }

        // Process request
        auto status_or_response = state->server->HandleRequest(deserialized_request);
        if (!status_or_response.ok()) {
            return PIR_ERROR_PROCESSING;
        }

        // Serialize response
        std::string serialized_response;
        if (!status_or_response.value().SerializeToString(&serialized_response)) {
            return PIR_ERROR_PROCESSING;
        }

        // Encode to base64
        std::string serialized_response_base64 = base64_encode(
            reinterpret_cast<const unsigned char*>(serialized_response.data()),
            serialized_response.size());

        // Allocate and copy response
        *response_base64 = strdup(serialized_response_base64.c_str());
        if (!*response_base64) {
            return PIR_ERROR_MEMORY;
        }

        return PIR_SUCCESS;

    } catch (const std::exception& e) {
        return PIR_ERROR_PROCESSING;
    }
}

void pir_server_destroy(void* server_handle) {
    if (server_handle) {
        auto state = static_cast<ServerState*>(server_handle);
        delete state;
    }
}

void pir_server_free_string(char* str) {
    free(str);
}

} // extern "C"