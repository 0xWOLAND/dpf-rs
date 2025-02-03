#include "external/google_dpf/pir/private_information_retrieval.pb.h"
#include "external/google_dpf/pir/prng/aes_128_ctr_seeded_prng.h"
#include "external/google_dpf/pir/testing/request_generator.h"
#include "external/google_dpf/pir/testing/mock_pir_database.h"
#include "external/google_dpf/pir/dense_dpf_pir_database.h"
#include "external/google_dpf/pir/dense_dpf_pir_server.h"
#include "nlohmann/json.hpp"
#include "base64_utils.h"
#include "server.h"

#include <memory>
#include <string>
#include <vector>
#include <cstring>

using namespace distributed_point_functions;

// Constants
constexpr int kBitsPerBlock = 128;

// Opaque struct to hold server state
struct PirServerWrapper {
    std::unique_ptr<DenseDpfPirServer> server;
    std::unique_ptr<DenseDpfPirServer::Database> database;
    std::unique_ptr<DistributedPointFunction> dpf;
    std::vector<std::string> elements;
    PirConfig config;
    DpfParameters params;
};

extern "C" {

// Create a new PIR server instance
PirServerWrapper* pir_server_create(int database_size, const char** elements, int num_elements) {
    auto wrapper = new PirServerWrapper();
    
    try {
        // Setup config
        wrapper->config.mutable_dense_dpf_pir_config()->set_num_elements(database_size);

        // Setup DPF parameters
        wrapper->params.mutable_value_type()->mutable_xor_wrapper()->set_bitsize(kBitsPerBlock);
        wrapper->params.set_log_domain_size(
            static_cast<int>(std::ceil(std::log2(database_size))));

        // Create DPF instance
        auto status_or_dpf = DistributedPointFunction::Create(wrapper->params);
        if (!status_or_dpf.ok()) {
            delete wrapper;
            return nullptr;
        }
        wrapper->dpf = std::move(status_or_dpf.value());

        // Setup database elements
        if (elements && num_elements > 0) {
            wrapper->elements.reserve(num_elements);
            for (int i = 0; i < num_elements; i++) {
                wrapper->elements.push_back(elements[i]);
            }
        } else {
            auto status_or_elements = pir_testing::GenerateCountingStrings(database_size, "Element ");
            if (!status_or_elements.ok()) {
                delete wrapper;
                return nullptr;
            }
            wrapper->elements = std::move(status_or_elements.value());
        }

        // Create database
        auto status_or_database = pir_testing::CreateFakeDatabase<DenseDpfPirDatabase>(wrapper->elements);
        if (!status_or_database.ok()) {
            delete wrapper;
            return nullptr;
        }
        wrapper->database = std::move(status_or_database.value());

        // Create server
        auto status_or_server = DenseDpfPirServer::CreatePlain(wrapper->config, std::move(wrapper->database));
        if (!status_or_server.ok()) {
            delete wrapper;
            return nullptr;
        }
        wrapper->server = std::move(status_or_server.value());

        return wrapper;
    } catch (const std::exception& e) {
        delete wrapper;
        return nullptr;
    }
}

// Process a PIR request
char* pir_server_handle_request(PirServerWrapper* wrapper, const char* serialized_request_base64) {
    if (!wrapper || !serialized_request_base64) {
        return nullptr;
    }

    try {
        // Decode base64 request
        std::string serialized_request = base64_decode(serialized_request_base64);
        
        // Deserialize request
        PirRequest deserialized_request;
        if (!deserialized_request.ParseFromString(serialized_request)) {
            return nullptr;
        }

        // Process request
        auto status_or_response = wrapper->server->HandleRequest(deserialized_request);
        if (!status_or_response.ok()) {
            return nullptr;
        }

        // Serialize response
        std::string serialized_response;
        status_or_response.value().SerializeToString(&serialized_response);

        // Encode to base64
        std::string serialized_response_base64 = base64_encode(
            reinterpret_cast<const unsigned char*>(serialized_response.data()),
            serialized_response.size());

        return strdup(serialized_response_base64.c_str());
    } catch (const std::exception& e) {
        return nullptr;
    }
}

// Free a response string
void pir_server_free_string(char* str) {
    free(str);
}

// Destroy the server instance
void pir_server_destroy(PirServerWrapper* wrapper) {
    delete wrapper;
}

} // extern "C"