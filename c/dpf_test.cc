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
    server1_ = pir_server_create(test_elements_.size(), elements, test_elements_.size());
    server2_ = pir_server_create(test_elements_.size(), elements, test_elements_.size());
    ASSERT_NE(server1_, nullptr);
    ASSERT_NE(server2_, nullptr);

    // Create client
    client_ = pir_client_create(test_elements_.size());
    ASSERT_NE(client_, nullptr);
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

  PirServerWrapper* server1_ = nullptr;
  PirServerWrapper* server2_ = nullptr;
  PirClientWrapper* client_ = nullptr;
  std::vector<std::string> test_elements_;
};

TEST_F(PirE2ETest, SingleElementQuery) {
  // Generate request for a single element
  int index = 1;  // Query for "Element1"
  char* requests = pir_client_generate_requests(client_, &index, 1);
  ASSERT_NE(requests, nullptr);

  // Parse requests JSON
  nlohmann::json requests_json = nlohmann::json::parse(requests);
  ASSERT_TRUE(requests_json.contains("request1"));
  ASSERT_TRUE(requests_json.contains("request2"));

  // Get responses from both servers
  char* response1 = pir_server_handle_request(server1_, requests_json["request1"].get<std::string>().c_str());
  char* response2 = pir_server_handle_request(server2_, requests_json["request2"].get<std::string>().c_str());
  ASSERT_NE(response1, nullptr);
  ASSERT_NE(response2, nullptr);

  // Create response JSON
  nlohmann::json response_json;
  response_json["response1"] = response1;
  response_json["response2"] = response2;

  // Process responses
  char* result = pir_client_process_responses(response_json.dump().c_str());
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
  char* requests = pir_client_generate_requests(client_, indices.data(), indices.size());
  ASSERT_NE(requests, nullptr);

  // Parse requests JSON
  nlohmann::json requests_json = nlohmann::json::parse(requests);

  // Get responses from both servers
  char* response1 = pir_server_handle_request(server1_, requests_json["request1"].get<std::string>().c_str());
  char* response2 = pir_server_handle_request(server2_, requests_json["request2"].get<std::string>().c_str());
  ASSERT_NE(response1, nullptr);
  ASSERT_NE(response2, nullptr);

  // Create response JSON
  nlohmann::json response_json;
  response_json["response1"] = response1;
  response_json["response2"] = response2;

  // Process responses
  char* result = pir_client_process_responses(response_json.dump().c_str());
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
  auto gen_server1 = pir_server_create(100, nullptr, 0);
  auto gen_server2 = pir_server_create(100, nullptr, 0);
  auto gen_client = pir_client_create(100);
  ASSERT_NE(gen_server1, nullptr);
  ASSERT_NE(gen_server2, nullptr);
  ASSERT_NE(gen_client, nullptr);

  // Generate request
  int index = 5;
  char* requests = pir_client_generate_requests(gen_client, &index, 1);
  ASSERT_NE(requests, nullptr);

  // Parse requests JSON
  nlohmann::json requests_json = nlohmann::json::parse(requests);

  // Get responses
  char* response1 = pir_server_handle_request(gen_server1, requests_json["request1"].get<std::string>().c_str());
  char* response2 = pir_server_handle_request(gen_server2, requests_json["request2"].get<std::string>().c_str());
  ASSERT_NE(response1, nullptr);
  ASSERT_NE(response2, nullptr);

  // Create response JSON
  nlohmann::json response_json;
  response_json["response1"] = response1;
  response_json["response2"] = response2;

  // Process responses
  char* result = pir_client_process_responses(response_json.dump().c_str());
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