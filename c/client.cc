#include "client.h"
#include "external/google_dpf/pir/private_information_retrieval.pb.h"
#include "external/google_dpf/pir/prng/aes_128_ctr_seeded_prng.h"
#include "external/google_dpf/pir/testing/request_generator.h"
#include "external/google_dpf/pir/dense_dpf_pir_client.h"
#include "nlohmann/json.hpp"
#include "base64_utils.h"

#include <memory>
#include <string>
#include <vector>
#include <mutex>
#include <cstring>

using namespace distributed_point_functions;

namespace {
    std::mutex g_mutex;
    std::string g_last_error;
    bool g_initialized = false;
}

// Internal client state structure
struct ClientState {
    std::unique_ptr<pir_testing::RequestGenerator> request_generator;
};

// Set last error message thread-safely
static void set_last_error(const std::string& error) {
    std::lock_guard<std::mutex> lock(g_mutex);
    g_last_error = error;
}

extern "C" {

pir_status_t pir_client_initialize(void) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (g_initialized) {
        return PIR_SUCCESS;
    }
    g_initialized = true;
    return PIR_SUCCESS;
}

void pir_client_cleanup(void) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (!g_initialized) {
        return;
    }
    g_initialized = false;
}

pir_status_t pir_client_create(int database_size, void** client_handle) {
    if (!g_initialized) {
        set_last_error("PIR client system not initialized");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    if (database_size <= 0 || !client_handle) {
        set_last_error("Invalid arguments");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = new ClientState();
        auto status_or_generator = pir_testing::RequestGenerator::Create(
            database_size, DenseDpfPirServer::kEncryptionContextInfo);
        
        if (!status_or_generator.ok()) {
            set_last_error("Failed to create request generator");
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        
        state->request_generator = std::move(status_or_generator.value());
        *client_handle = state;
        return PIR_SUCCESS;
    } catch (const std::exception& e) {
        set_last_error(std::string("Exception: ") + e.what());
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_client_generate_requests(void* client_handle, const int* indices, 
                                        int num_indices, char** requests_json) {
    if (!g_initialized) {
        set_last_error("PIR client system not initialized");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    if (!client_handle || !indices || num_indices <= 0 || !requests_json) {
        set_last_error("Invalid arguments");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = static_cast<ClientState*>(client_handle);
        std::vector<int> indices_vec(indices, indices + num_indices);
        
        PirRequest request1, request2;
        auto status_or_requests = state->request_generator->CreateDpfPirPlainRequests(indices_vec);
        
        if (!status_or_requests.ok()) {
            set_last_error("Failed to create PIR requests");
            return PIR_ERROR_PROCESSING;
        }

        std::tie(*request1.mutable_dpf_pir_request()->mutable_plain_request(),
                *request2.mutable_dpf_pir_request()->mutable_plain_request()) = 
                std::move(status_or_requests.value());

        // Serialize requests
        std::string serialized_request1, serialized_request2;
        if (!request1.SerializeToString(&serialized_request1) ||
            !request2.SerializeToString(&serialized_request2)) {
            set_last_error("Failed to serialize requests");
            return PIR_ERROR_PROCESSING;
        }

        // Base64 encode requests
        std::string encoded_request1 = base64_encode(
            reinterpret_cast<const unsigned char*>(serialized_request1.c_str()), 
            serialized_request1.length());
        std::string encoded_request2 = base64_encode(
            reinterpret_cast<const unsigned char*>(serialized_request2.c_str()),
            serialized_request2.length());

        // Create JSON object
        nlohmann::json j;
        j["request1"] = encoded_request1;
        j["request2"] = encoded_request2;

        std::string json_str = j.dump();
        *requests_json = strdup(json_str.c_str());
        
        if (!*requests_json) {
            set_last_error("Memory allocation failed");
            return PIR_ERROR_MEMORY;
        }

        return PIR_SUCCESS;
    } catch (const std::exception& e) {
        set_last_error(std::string("Exception: ") + e.what());
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_client_process_responses(const char* responses_json, char** merged_result) {
    if (!g_initialized) {
        set_last_error("PIR client system not initialized");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    if (!responses_json || !merged_result) {
        set_last_error("Invalid arguments");
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        nlohmann::json responses_json_obj = nlohmann::json::parse(responses_json);
        
        if (!responses_json_obj.contains("response1") || !responses_json_obj.contains("response2")) {
            set_last_error("Invalid JSON format: missing response fields");
            return PIR_ERROR_INVALID_ARGUMENT;
        }

        std::string serialized_response1_base64 = responses_json_obj["response1"];
        std::string serialized_response2_base64 = responses_json_obj["response2"];
        
        std::string serialized_response1 = base64_decode(serialized_response1_base64);
        std::string serialized_response2 = base64_decode(serialized_response2_base64);
        
        PirResponse deserialized_response1, deserialized_response2;
        if (!deserialized_response1.ParseFromString(serialized_response1) ||
            !deserialized_response2.ParseFromString(serialized_response2)) {
            set_last_error("Failed to parse responses");
            return PIR_ERROR_PROCESSING;
        }

        if (deserialized_response1.dpf_pir_response().masked_response_size() !=
            deserialized_response2.dpf_pir_response().masked_response_size()) {
            set_last_error("Response size mismatch");
            return PIR_ERROR_PROCESSING;
        }

        std::vector<std::string> result;
        for (int i = 0; i < deserialized_response1.dpf_pir_response().masked_response_size(); i++) {
            if (deserialized_response1.dpf_pir_response().masked_response(i).size() !=
                deserialized_response2.dpf_pir_response().masked_response(i).size()) {
                set_last_error("Response element size mismatch");
                return PIR_ERROR_PROCESSING;
            }

            result.emplace_back(
                deserialized_response1.dpf_pir_response().masked_response(i).size(), '\0');
            
            for (int j = 0; j < deserialized_response1.dpf_pir_response().masked_response(i).size(); ++j) {
                result.back()[j] =
                    deserialized_response1.dpf_pir_response().masked_response(i)[j] ^
                    deserialized_response2.dpf_pir_response().masked_response(i)[j];
            }
        }

        // Join results with commas
        std::string final_result;
        for (size_t i = 0; i < result.size(); ++i) {
            final_result += result[i];
            if (i < result.size() - 1) {
                final_result += ", ";
            }
        }

        *merged_result = strdup(final_result.c_str());
        if (!*merged_result) {
            set_last_error("Memory allocation failed");
            return PIR_ERROR_MEMORY;
        }

        return PIR_SUCCESS;
    } catch (const std::exception& e) {
        set_last_error(std::string("Exception: ") + e.what());
        return PIR_ERROR_PROCESSING;
    }
}

void pir_client_free_string(char* str) {
    free(str);
}

void pir_client_destroy(void* client_handle) {
    if (client_handle) {
        auto state = static_cast<ClientState*>(client_handle);
        delete state;
    }
}

const char* pir_client_get_last_error(void) {
    std::lock_guard<std::mutex> lock(g_mutex);
    return g_last_error.c_str();
}

} // extern "C"