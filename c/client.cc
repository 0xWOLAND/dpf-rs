#include "client.h"
#include "external/google_dpf/pir/private_information_retrieval.pb.h"
#include "external/google_dpf/pir/prng/aes_128_ctr_seeded_prng.h"
#include "external/google_dpf/pir/dense_dpf_pir_client.h"
#include "external/google_dpf/dpf/distributed_point_function.h"
#include "nlohmann/json.hpp"
#include "base64_utils.h"

#include <memory>
#include <string>
#include <vector>
#include <mutex>
#include <cstring>
#include <cmath>

using namespace distributed_point_functions;

constexpr int kBitsPerBlock = 128;

// Internal client state structure
struct ClientState {
    std::unique_ptr<DistributedPointFunction> dpf;
    std::string otp_seed;
    std::string encryption_context_info;
    int database_size;
    DpfParameters params;
};

extern "C" {

pir_status_t pir_client_create(int database_size, void** client_handle) {
    if (database_size <= 0 || !client_handle) {
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = new ClientState();
        state->database_size = database_size;
        state->encryption_context_info = std::string(DenseDpfPirServer::kEncryptionContextInfo);

        // Setup DPF parameters
        state->params.mutable_value_type()->mutable_xor_wrapper()->set_bitsize(kBitsPerBlock);
        state->params.set_log_domain_size(
            static_cast<int>(std::ceil(std::log2(database_size))));

        // Create DPF instance
        auto status_or_dpf = DistributedPointFunction::Create(state->params);
        if (!status_or_dpf.ok()) {
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->dpf = std::move(status_or_dpf.value());

        // Generate OTP seed
        auto status_or_seed = Aes128CtrSeededPrng::GenerateSeed();
        if (!status_or_seed.ok()) {
            delete state;
            return PIR_ERROR_PROCESSING;
        }
        state->otp_seed = std::move(status_or_seed.value());

        *client_handle = state;
        return PIR_SUCCESS;
    } catch (const std::exception& e) {
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_client_generate_requests(void* client_handle, const int* indices, 
                                        int num_indices, char** requests_json) {
    if (!client_handle || !indices || num_indices <= 0 || !requests_json) {
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        auto state = static_cast<ClientState*>(client_handle);
        std::vector<int> indices_vec(indices, indices + num_indices);
        
        // Create plain requests
        DpfPirRequest::PlainRequest request1, request2;
        for (int index : indices_vec) {
            if (index < 0 || index >= state->database_size) {
                return PIR_ERROR_INVALID_ARGUMENT;
            }

            absl::uint128 alpha = index / kBitsPerBlock;
            XorWrapper<absl::uint128> beta(absl::uint128{1} << (index % kBitsPerBlock));

            auto status_or_keys = state->dpf->GenerateKeys(alpha, beta);
            if (!status_or_keys.ok()) {
                return PIR_ERROR_PROCESSING;
            }

            auto& [key1, key2] = status_or_keys.value();
            *request1.mutable_dpf_key()->Add() = std::move(key1);
            *request2.mutable_dpf_key()->Add() = std::move(key2);
        }

        // Create PIR requests
        PirRequest pir_request1, pir_request2;
        *pir_request1.mutable_dpf_pir_request()->mutable_plain_request() = std::move(request1);
        *pir_request2.mutable_dpf_pir_request()->mutable_plain_request() = std::move(request2);

        // Serialize requests
        std::string serialized_request1, serialized_request2;
        if (!pir_request1.SerializeToString(&serialized_request1) ||
            !pir_request2.SerializeToString(&serialized_request2)) {
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
            return PIR_ERROR_MEMORY;
        }

        return PIR_SUCCESS;
    } catch (const std::exception& e) {
        return PIR_ERROR_PROCESSING;
    }
}

pir_status_t pir_client_process_responses(const char* responses_json, char** merged_result) {
    if (!responses_json || !merged_result) {
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    try {
        nlohmann::json responses_json_obj = nlohmann::json::parse(responses_json);
        
        if (!responses_json_obj.contains("response1") || !responses_json_obj.contains("response2")) {
            return PIR_ERROR_INVALID_ARGUMENT;
        }

        std::string serialized_response1_base64 = responses_json_obj["response1"];
        std::string serialized_response2_base64 = responses_json_obj["response2"];
        
        std::string serialized_response1 = base64_decode(serialized_response1_base64);
        std::string serialized_response2 = base64_decode(serialized_response2_base64);
        
        PirResponse deserialized_response1, deserialized_response2;
        if (!deserialized_response1.ParseFromString(serialized_response1) ||
            !deserialized_response2.ParseFromString(serialized_response2)) {
            return PIR_ERROR_PROCESSING;
        }

        // Process responses by XORing them together
        if (deserialized_response1.dpf_pir_response().masked_response_size() !=
            deserialized_response2.dpf_pir_response().masked_response_size()) {
            return PIR_ERROR_PROCESSING;
        }

        std::vector<std::string> result;
        for (int i = 0; i < deserialized_response1.dpf_pir_response().masked_response_size(); i++) {
            if (deserialized_response1.dpf_pir_response().masked_response(i).size() !=
                deserialized_response2.dpf_pir_response().masked_response(i).size()) {
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
            return PIR_ERROR_MEMORY;
        }

        return PIR_SUCCESS;
    } catch (const std::exception& e) {
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

} // extern "C"