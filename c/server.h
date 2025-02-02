#ifndef DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_SERVER_C_H_
#define DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_SERVER_C_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to the DPF PIR server
typedef struct DpfPirServer_st* DpfPirServer;

// Buffer structure for data transfer
typedef struct {
    uint8_t* data;
    size_t size;
} DpfPirBuffer;

// Status codes
typedef enum {
    DPF_PIR_OK = 0,
    DPF_PIR_INVALID_ARGUMENT = 1,
    DPF_PIR_FAILED_PRECONDITION = 2,
    DPF_PIR_OUT_OF_MEMORY = 3,
    DPF_PIR_INTERNAL_ERROR = 4
} DpfPirStatus;

// Server configuration
typedef struct {
    uint64_t num_elements;   // Number of elements in the database
} DpfPirConfig;

// Request structure
typedef struct {
    DpfPirBuffer leader_request;  // For leader server
    DpfPirBuffer helper_request;  // For helper server
} DpfPirRequest;

// Response structure
typedef struct {
    DpfPirBuffer response;
} DpfPirResponse;

// Callback type for forwarding helper requests (used by leader)
typedef DpfPirStatus (*DpfPirForwardHelperRequestFn)(
    const DpfPirBuffer* request,
    DpfPirBuffer* response,
    void* user_data);

// Callback type for decrypting helper requests
typedef DpfPirStatus (*DpfPirDecryptHelperRequestFn)(
    const DpfPirBuffer* ciphertext,
    const char* context_info,
    DpfPirBuffer* plaintext,
    void* user_data);

// Server creation functions
DpfPirStatus dpf_pir_server_create_leader(
    const DpfPirConfig* config,
    void* database,             // Database handle from dpf_pir_database_c.h
    DpfPirForwardHelperRequestFn forward_fn,
    void* user_data,
    DpfPirServer* server);

DpfPirStatus dpf_pir_server_create_helper(
    const DpfPirConfig* config,
    void* database,             // Database handle from dpf_pir_database_c.h
    DpfPirDecryptHelperRequestFn decrypt_fn,
    void* user_data,
    DpfPirServer* server);

DpfPirStatus dpf_pir_server_create_plain(
    const DpfPirConfig* config,
    void* database,             // Database handle from dpf_pir_database_c.h
    DpfPirServer* server);

// Handle PIR request
DpfPirStatus dpf_pir_server_handle_request(
    DpfPirServer server,
    const DpfPirRequest* request,
    DpfPirResponse* response);

// Memory management
void dpf_pir_server_destroy(DpfPirServer server);
void dpf_pir_buffer_free(DpfPirBuffer* buffer);
void dpf_pir_request_free(DpfPirRequest* request);
void dpf_pir_response_free(DpfPirResponse* response);

// Error handling
const char* dpf_pir_get_last_error(void);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_SERVER_C_H_