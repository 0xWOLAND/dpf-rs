#ifndef PIR_CLIENT_H
#define PIR_CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif

#include "status.h"

// Initialize the PIR client system. Must be called before creating any clients
pir_status_t pir_client_initialize(void);

// Cleanup the PIR client system. Should be called when completely done with PIR
void pir_client_cleanup(void);

// Create a new PIR client instance
pir_status_t pir_client_create(
    int database_size,
    void** client_handle
);

// Generate PIR requests for given indices
// requests_json will be allocated by the function and must be freed with pir_client_free_string
pir_status_t pir_client_generate_requests(
    void* client_handle,
    const int* indices,
    int num_indices,
    char** requests_json
);

// Process responses from both servers
// merged_result will be allocated by the function and must be freed with pir_client_free_string
pir_status_t pir_client_process_responses(
    const char* responses_json,
    char** merged_result
);

// Free a string allocated by the PIR client system
void pir_client_free_string(char* str);

// Destroy a client instance
void pir_client_destroy(void* client_handle);

// Get the last error message
const char* pir_client_get_last_error(void);

#ifdef __cplusplus
}
#endif

#endif // PIR_CLIENT_H