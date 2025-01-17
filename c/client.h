#ifndef DPF_PIR_CLIENT_H_
#define DPF_PIR_CLIENT_H_

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

// Opaque handle to the client
typedef struct DpfPirClient_st* DpfPirClient;

// Status codes for operations
typedef enum {
    DPF_PIR_OK = 0,
    DPF_PIR_INVALID_ARGUMENT = 1,
    DPF_PIR_FAILED_PRECONDITION = 2,
    DPF_PIR_OUT_OF_MEMORY = 3,
    DPF_PIR_INTERNAL_ERROR = 4
} DpfPirStatus;

// Request structure
typedef struct {
    uint8_t* data;
    size_t size;
} DpfPirBuffer;

typedef struct {
    DpfPirBuffer leader_request;
    DpfPirBuffer helper_request;
    DpfPirBuffer client_state;
} DpfPirRequest;

// Configuration structure
typedef struct {
    int32_t database_size;
    const char* encryption_context;
} DpfPirConfig;

// Creates a new DPF PIR client
DpfPirStatus dpf_pir_client_create(
    const DpfPirConfig* config,
    DpfPirClient* client);

// Destroys a DPF PIR client
void dpf_pir_client_destroy(DpfPirClient client);

// Creates a request for specific indices
DpfPirStatus dpf_pir_client_create_request(
    DpfPirClient client,
    const int32_t* indices,
    size_t num_indices,
    DpfPirRequest* request);

// Frees memory associated with a request
void dpf_pir_request_free(DpfPirRequest* request);

// Handles response from the server
DpfPirStatus dpf_pir_client_handle_response(
    DpfPirClient client,
    const DpfPirBuffer* response,
    const DpfPirBuffer* client_state,
    DpfPirBuffer* result);

// Frees a buffer
void dpf_pir_buffer_free(DpfPirBuffer* buffer);

// Gets the last error message
const char* dpf_pir_get_last_error(void);

#ifdef __cplusplus
}
#endif

#endif  // DPF_PIR_CLIENT_H_