#include "external/google_dpf/pir/private_information_retrieval.pb.h"
#include "external/google_dpf/pir/prng/aes_128_ctr_seeded_prng.h"
#include "external/google_dpf/pir/testing/request_generator.h"
#include "external/google_dpf/pir/dense_dpf_pir_client.h"
#include "nlohmann/json.hpp"
#include "base64_utils.h"
#include "client.h"

#include <memory>
#include <string>
#include <vector>
#include <cstring>

using namespace distributed_point_functions;

// Opaque struct to hold client state
struct PirClientWrapper {
    std::unique_ptr<pir_testing::RequestGenerator> request_generator;
};

extern "C" {

// Create a new PIR client instance
PirClientWrapper* pir_client_create(int database_size) {
    auto client = new PirClientWrapper();
    auto status_or_generator = pir_testing::RequestGenerator::Create(
        database_size, DenseDpfPirServer::kEncryptionContextInfo);
    
    if (!status_or_generator.ok()) {
        delete client;
        return nullptr;
    }
    
    client->request_generator = std::move(status_or_generator.value());
    return client;
}

// Generate PIR requests for given indices
// Returns a JSON string containing both requests that must be freed by caller
char* pir_client_generate_requests(PirClientWrapper* client, const int* indices, int num_indices) {
    if (!client || !indices || num_indices <= 0) {
        return nullptr;
    }

    try {
        std::vector<int> indices_vec(indices, indices + num_indices);
        
        PirRequest request1, request2;
        auto status_or_requests = client->request_generator->CreateDpfPirPlainRequests(indices_vec);
        
        if (!status_or_requests.ok()) {
            return nullptr;
        }

        std::tie(*request1.mutable_dpf_pir_request()->mutable_plain_request(),
                *request2.mutable_dpf_pir_request()->mutable_plain_request()) = 
                std::move(status_or_requests.value());

        // Serialize requests
        std::string serialized_request1, serialized_request2;
        request1.SerializeToString(&serialized_request1);
        request2.SerializeToString(&serialized_request2);

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
        char* result = static_cast<char*>(malloc(json_str.length() + 1));
        strcpy(result, json_str.c_str());
        return result;
    } catch (const std::exception& e) {
        return nullptr;
    }
}

// Process responses from both servers
// Returns the merged result that must be freed by caller
char* pir_client_process_responses(const char* serialized_responses) {
    if (!serialized_responses) {
        return nullptr;
    }

    try {
        nlohmann::json responses_json = nlohmann::json::parse(serialized_responses);
        
        std::string serialized_response1_base64 = responses_json["response1"];
        std::string serialized_response2_base64 = responses_json["response2"];
        
        std::string serialized_response1 = base64_decode(serialized_response1_base64);
        std::string serialized_response2 = base64_decode(serialized_response2_base64);
        
        PirResponse deserialized_response1, deserialized_response2;
        if (!deserialized_response1.ParseFromString(serialized_response1) ||
            !deserialized_response2.ParseFromString(serialized_response2)) {
            return nullptr;
        }

        if (deserialized_response1.dpf_pir_response().masked_response_size() !=
            deserialized_response2.dpf_pir_response().masked_response_size()) {
            return nullptr;
        }

        std::vector<std::string> result;
        for (int i = 0; i < deserialized_response1.dpf_pir_response().masked_response_size(); i++) {
            if (deserialized_response1.dpf_pir_response().masked_response(i).size() !=
                deserialized_response2.dpf_pir_response().masked_response(i).size()) {
                return nullptr;
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

        char* output = static_cast<char*>(malloc(final_result.length() + 1));
        strcpy(output, final_result.c_str());
        return output;
    } catch (const std::exception& e) {
        return nullptr;
    }
}

// Free memory allocated by the client
void pir_client_free_string(char* str) {
    free(str);
}

// Destroy the client instance
void pir_client_destroy(PirClientWrapper* client) {
    delete client;
}

} // extern "C"