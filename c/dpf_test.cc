#include "client.h"
#include "server.h"
#include "gtest/gtest.h"
#include "nlohmann/json.hpp"
#include "base64_utils.h"

#include <vector>
#include <string>
#include <memory>

namespace distributed_point_functions {
namespace {

pir_status_t create_test_server(int database_size, void** server_handle) {
    if (!server_handle) {
        return PIR_ERROR_INVALID_ARGUMENT;
    }

    std::vector<std::string> test_elements;
    test_elements.reserve(database_size);
    for (int i = 0; i < database_size; i++) {
        test_elements.push_back("Element " + std::to_string(i));
    }

    std::vector<const char*> element_ptrs;
    element_ptrs.reserve(database_size);
    for (const auto& elem : test_elements) {
        element_ptrs.push_back(elem.c_str());
    }

    return pir_server_create(element_ptrs.data(), database_size, server_handle);
}

class PirE2ETest : public ::testing::Test {
 protected:
  void SetUp() override {
    test_elements_ = {"Element0", "Element1", "Element2", "Element3"};
    const char* elements[4] = {
        test_elements_[0].c_str(),
        test_elements_[1].c_str(),
        test_elements_[2].c_str(),
        test_elements_[3].c_str()
    };
    
    // Create two servers
    pir_status_t status = pir_server_create(elements, test_elements_.size(), &server1_);
    ASSERT_EQ(status, PIR_SUCCESS);
    
    status = pir_server_create(elements, test_elements_.size(), &server2_);
    ASSERT_EQ(status, PIR_SUCCESS);

    // Create client
    status = pir_client_create(test_elements_.size(), &client_);
    ASSERT_EQ(status, PIR_SUCCESS);
  }

  void TearDown() override {
    if (server1_) {
      pir_server_destroy(server1_);
      server1_ = nullptr;
    }
    if (server2_) {
      pir_server_destroy(server2_);
      server2_ = nullptr;
    }
    if (client_) {
      pir_client_destroy(client_);
      client_ = nullptr;
    }
  }

  void* server1_ = nullptr;
  void* server2_ = nullptr;
  void* client_ = nullptr;
  std::vector<std::string> test_elements_;
};

TEST_F(PirE2ETest, SingleElementQuery) {
  // Generate request for a single element
  int index = 1;  // Query for "Element1"
  char* requests = nullptr;
  pir_status_t status = pir_client_generate_requests(client_, &index, 1, &requests);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(requests, nullptr);

  // Parse requests JSON
  nlohmann::json requests_json = nlohmann::json::parse(requests);
  ASSERT_TRUE(requests_json.contains("request1"));
  ASSERT_TRUE(requests_json.contains("request2"));

  // Get responses from both servers
  char* response1 = nullptr;
  char* response2 = nullptr;
  status = pir_server_process_request(server1_, requests_json["request1"].get<std::string>().c_str(), &response1);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(response1, nullptr);

  status = pir_server_process_request(server2_, requests_json["request2"].get<std::string>().c_str(), &response2);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(response2, nullptr);

  // Create response JSON
  nlohmann::json response_json;
  response_json["response1"] = response1;
  response_json["response2"] = response2;

  // Process responses
  char* result = nullptr;
  status = pir_client_process_responses(response_json.dump().c_str(), &result);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(result, nullptr);
  EXPECT_EQ(std::string(result), "Element1");

  // Cleanup
  pir_client_free_string(requests);
  pir_server_free_string(response1);
  pir_server_free_string(response2);
  pir_client_free_string(result);
}

TEST_F(PirE2ETest, MultiElementQuery) {
  // Generate request for multiple elements
  std::vector<int> indices = {0, 2};  // Query for "Element0" and "Element2"
  char* requests = nullptr;
  pir_status_t status = pir_client_generate_requests(client_, indices.data(), indices.size(), &requests);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(requests, nullptr);

  // Parse requests JSON
  nlohmann::json requests_json = nlohmann::json::parse(requests);

  // Get responses from both servers
  char* response1 = nullptr;
  char* response2 = nullptr;
  status = pir_server_process_request(server1_, requests_json["request1"].get<std::string>().c_str(), &response1);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(response1, nullptr);

  status = pir_server_process_request(server2_, requests_json["request2"].get<std::string>().c_str(), &response2);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(response2, nullptr);

  // Create response JSON
  nlohmann::json response_json;
  response_json["response1"] = response1;
  response_json["response2"] = response2;

  // Process responses
  char* result = nullptr;
  status = pir_client_process_responses(response_json.dump().c_str(), &result);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(result, nullptr);
  EXPECT_EQ(std::string(result), "Element0, Element2");

  // Cleanup
  pir_client_free_string(requests);
  pir_server_free_string(response1);
  pir_server_free_string(response2);
  pir_client_free_string(result);
}

TEST_F(PirE2ETest, GeneratedDataQuery) {
  // Create new servers and client with generated data
  void* gen_server1 = nullptr;
  void* gen_server2 = nullptr;
  void* gen_client = nullptr;

  pir_status_t status = create_test_server(100, &gen_server1);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(gen_server1, nullptr);

  status = create_test_server(100, &gen_server2);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(gen_server2, nullptr);

  status = pir_client_create(100, &gen_client);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(gen_client, nullptr);

  // Generate request
  int index = 5;
  char* requests = nullptr;
  status = pir_client_generate_requests(gen_client, &index, 1, &requests);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(requests, nullptr);

  // Parse requests JSON
  nlohmann::json requests_json = nlohmann::json::parse(requests);

  // Get responses
  char* response1 = nullptr;
  char* response2 = nullptr;
  status = pir_server_process_request(gen_server1, requests_json["request1"].get<std::string>().c_str(), &response1);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(response1, nullptr);

  status = pir_server_process_request(gen_server2, requests_json["request2"].get<std::string>().c_str(), &response2);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(response2, nullptr);

  // Create response JSON
  nlohmann::json response_json;
  response_json["response1"] = response1;
  response_json["response2"] = response2;

  // Process responses
  char* result = nullptr;
  status = pir_client_process_responses(response_json.dump().c_str(), &result);
  ASSERT_EQ(status, PIR_SUCCESS);
  ASSERT_NE(result, nullptr);
  EXPECT_EQ(std::string(result), "Element 5");

  // Cleanup
  pir_client_free_string(requests);
  pir_server_free_string(response1);
  pir_server_free_string(response2);
  pir_client_free_string(result);
  pir_server_destroy(gen_server1);
  pir_server_destroy(gen_server2);
  pir_client_destroy(gen_client);
}

}  // namespace
}  // namespace distributed_point_functions

int main(int argc, char** argv) {
  ::testing::InitGoogleTest(&argc, argv);
  return RUN_ALL_TESTS();
}