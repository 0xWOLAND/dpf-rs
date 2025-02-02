#ifndef DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_CLIENT_C_H_
#define DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_CLIENT_C_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to the DPF PIR client
typedef struct DpfPirClient_st* DpfPirClient;

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

// Client configuration
typedef struct {
    uint64_t database_size;
} DpfPirConfig;

// Request state and data
typedef struct {
    DpfPirBuffer leader_request;
    DpfPirBuffer helper_request;
    DpfPirBuffer client_state;  // needed to handle the response
} DpfPirRequest;

// Response structure
typedef struct {
    char** values;      // Array of retrieved values
    size_t* lengths;    // Array of value lengths
    size_t num_values;  // Number of values in the response
} DpfPirResponse;

// Callback for encrypting helper requests
typedef DpfPirStatus (*DpfPirEncryptRequestFn)(
    const DpfPirBuffer* plaintext,
    const char* context_info,
    DpfPirBuffer* ciphertext,
    void* user_data);

// Client creation and management
DpfPirStatus dpf_pir_client_create(
    const DpfPirConfig* config,
    DpfPirEncryptRequestFn encrypt_fn,
    void* user_data,
    const char* encryption_context_info,
    DpfPirClient* client);

// Create a request for specific indices
DpfPirStatus dpf_pir_client_create_request(
    DpfPirClient client,
    const int32_t* indices,      // Array of indices to query
    size_t num_indices,          // Number of indices
    DpfPirRequest* request);     // Output request

// Handle server response
DpfPirStatus dpf_pir_client_handle_response(
    DpfPirClient client,
    const DpfPirBuffer* response,
    const DpfPirBuffer* client_state,
    DpfPirResponse* result);

// Memory management
void dpf_pir_client_destroy(DpfPirClient client);
void dpf_pir_request_free(DpfPirRequest* request);
void dpf_pir_response_free(DpfPirResponse* response);
void dpf_pir_buffer_free(DpfPirBuffer* buffer);

// Error handling
const char* dpf_pir_get_last_error(void);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_CLIENT_C_H_