#ifndef PIR_CLIENT_H
#define PIR_CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif

#include "status.h"

// Create a new PIR client instance
pir_status_t pir_client_create(
    int database_size,
    void** client_handle
);

// Generate PIR requests for given indices
pir_status_t pir_client_generate_requests(
    void* client_handle,
    const int* indices,
    int num_indices,
    char** requests_json
);

// Process responses from both servers
pir_status_t pir_client_process_responses(
    const char* responses_json,
    char** merged_result
);

// Free a string allocated by the PIR client system
void pir_client_free_string(char* str);

// Destroy a client instance
void pir_client_destroy(void* client_handle);

#ifdef __cplusplus
}
#endif

#endif // PIR_CLIENT_H