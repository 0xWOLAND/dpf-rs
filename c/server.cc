#include "server.h"
#include "external/google_dpf/pir/private_information_retrieval.pb.h"
#include "external/google_dpf/pir/prng/aes_128_ctr_seeded_prng.h"
#include "external/google_dpf/pir/testing/request_generator.h"
#include "external/google_dpf/pir/testing/mock_pir_database.h"
#include "external/google_dpf/pir/dense_dpf_pir_database.h"
#include "external/google_dpf/pir/dense_dpf_pir_server.h"
#include "base64_utils.h"

#include <memory>
#include <string>
#include <vector>
#include <mutex>
#include <thread>

using namespace distributed_point_functions;

// Constants
constexpr int kBitsPerBlock = 128;

// Global state
namespace {
    std::mutex g_mutex;
    std::string g_last_error;
    bool g_initialized = false;
}

// Internal server state structure
struct ServerState {
    std::unique_ptr<DenseDpfPirServer> server;
    std::unique_ptr<DenseDpfPirServer::Database> database;
    std::unique_ptr<DistributedPointFunction> dpf;
    std::vector<std::string> elements;
    PirConfig config;
    DpfParameters params;
};

// Set last error message thread-safely
static void set_last_error(const std::string& error) {
    std::lock_guard<std::mutex> lock(g_mutex);
    g_last_error = error;
}

extern "C" {

pir_status_t pir_initialize(void) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (g_initialized) {
        return PIR_SUCCESS;
    }
    
    // Add any necessary initialization here
    g_initialized = true;
    return PIR_SUCCESS;
}

void pir_cleanup(void) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (!g_initialized) {
        return;
    }
    
    // Add any necessary cleanup here
    g_initialized = false;
}

pir_status_t pir_server_create_test(int database_size, void** server_handle) {
    if (!g_initialized) {
        set_last_error("PIR system not initialized");
        return PIR_ERROR_INVALID_ARGUMENT;
    }
    
    if (database_size <= 0 || !server_handle) {
        set_last_error("Invalid arguments");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = new ServerState();
        
        // Setup config
        state->config.mutable_dense_dpf_pir_config()->set_num_elements(database_size);

        // Setup DPF parameters
        state->params.mutable_value_type()->mutable_xor_wrapper()->set_bitsize(kBitsPerBlock);
        state->params.set_log_domain_size(
            static_cast<int>(std::ceil(std::log2(database_size))));

        // Create DPF instance
        auto status_or_dpf = DistributedPointFunction::Create(state->params);
        if (!status_or_dpf.ok()) {
            set_last_error("Failed to create DPF");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->dpf = std::move(status_or_dpf.value());

        // Generate test elements
        auto status_or_elements = pir_testing::GenerateCountingStrings(database_size, "Element ");
        if (!status_or_elements.ok()) {
            set_last_error("Failed to generate test elements");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->elements = std::move(status_or_elements.value());

        // Create database
        auto status_or_database = pir_testing::CreateFakeDatabase<DenseDpfPirDatabase>(state->elements);
        if (!status_or_database.ok()) {
            set_last_error("Failed to create database");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->database = std::move(status_or_database.value());

        // Create server
        auto status_or_server = DenseDpfPirServer::CreatePlain(state->config, std::move(state->database));
        if (!status_or_server.ok()) {
            set_last_error("Failed to create server");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->server = std::move(status_or_server.value());

        *server_handle = state;
        return PIR_SUCCESS;

    } catch (const std::exception& e) {
        set_last_error(std::string("Exception: ") + e.what());
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_server_create(const char** elements, int num_elements, void** server_handle) {
    if (!g_initialized) {
        set_last_error("PIR system not initialized");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    if (!elements || num_elements <= 0 || !server_handle) {
        set_last_error("Invalid arguments");
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
            set_last_error("Failed to create DPF");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->dpf = std::move(status_or_dpf.value());

        // Copy elements
        state->elements.reserve(num_elements);
        for (int i = 0; i < num_elements; i++) {
            if (!elements[i]) {
                set_last_error("Invalid element pointer");
                delete state;
                return PIR_ERROR_INVALID_ARGUMENT;
            }
            state->elements.push_back(elements[i]);
        }

        // Create database
        auto status_or_database = pir_testing::CreateFakeDatabase<DenseDpfPirDatabase>(state->elements);
        if (!status_or_database.ok()) {
            set_last_error("Failed to create database");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->database = std::move(status_or_database.value());

        // Create server
        auto status_or_server = DenseDpfPirServer::CreatePlain(state->config, std::move(state->database));
        if (!status_or_server.ok()) {
            set_last_error("Failed to create server");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->server = std::move(status_or_server.value());

        *server_handle = state;
        return PIR_SUCCESS;

    } catch (const std::exception& e) {
        set_last_error(std::string("Exception: ") + e.what());
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_server_process_request(void* server_handle, const char* request_base64, char** response_base64) {
    if (!g_initialized) {
        set_last_error("PIR system not initialized");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    if (!server_handle || !request_base64 || !response_base64) {
        set_last_error("Invalid arguments");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = static_cast<ServerState*>(server_handle);

        // Decode base64 request
        std::string serialized_request = base64_decode(request_base64);
        
        // Deserialize request
        PirRequest deserialized_request;
        if (!deserialized_request.ParseFromString(serialized_request)) {
            set_last_error("Failed to parse request");
            return PIR_ERROR_PROCESSING;
        }

        // Process request
        auto status_or_response = state->server->HandleRequest(deserialized_request);
        if (!status_or_response.ok()) {
            set_last_error("Failed to process request");
            return PIR_ERROR_PROCESSING;
        }

        // Serialize response
        std::string serialized_response;
        if (!status_or_response.value().SerializeToString(&serialized_response)) {
            set_last_error("Failed to serialize response");
            return PIR_ERROR_PROCESSING;
        }

        // Encode to base64
        std::string serialized_response_base64 = base64_encode(
            reinterpret_cast<const unsigned char*>(serialized_response.data()),
            serialized_response.size());

        // Allocate and copy response
        *response_base64 = strdup(serialized_response_base64.c_str());
        if (!*response_base64) {
            set_last_error("Memory allocation failed");
            return PIR_ERROR_MEMORY;
        }

        return PIR_SUCCESS;

    } catch (const std::exception& e) {
        set_last_error(std::string("Exception: ") + e.what());
        return PIR_ERROR_PROCESSING;
    }
}

void pir_server_destroy(void* server_handle) {
    if (server_handle) {
        auto state = static_cast<ServerState*>(server_handle);
        delete state;
    }
}

const char* pir_get_last_error(void) {
    std::lock_guard<std::mutex> lock(g_mutex);
    return g_last_error.c_str();
}

void pir_server_free_string(char* str) {
    free(str);
}

} // extern "C"