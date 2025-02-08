#ifndef PIR_SERVER_H
#define PIR_SERVER_H

#ifdef __cplusplus
extern "C" {
#endif

#include "status.h"

// Create a new PIR server with provided elements
pir_status_t pir_server_create(
    const char** elements,
    int num_elements, 
    void** server_handle
);

// Process a PIR request
pir_status_t pir_server_process_request(
    void* server_handle,
    const char* request_base64,
    char** response_base64
);

// Free a string allocated by the PIR server
void pir_server_free_string(char* str);

// Destroy a server instance
void pir_server_destroy(void* server_handle);

#ifdef __cplusplus
}
#endif

#endif // PIR_SERVER_H