#ifndef PIR_SERVER_H
#define PIR_SERVER_H

#ifdef __cplusplus
extern "C" {
#endif

// Opaque pointer to server state
typedef struct PirServerWrapper PirServerWrapper;

// Create a new PIR server instance
// If elements is NULL or num_elements is 0, generates test data
PirServerWrapper* pir_server_create(int database_size, const char** elements, int num_elements);

// Process a PIR request
// Returns base64 encoded response that must be freed using pir_server_free_string
char* pir_server_handle_request(PirServerWrapper* wrapper, const char* serialized_request_base64);

// Free a response string
void pir_server_free_string(char* str);

// Destroy the server instance
void pir_server_destroy(PirServerWrapper* wrapper);

#ifdef __cplusplus
}
#endif

#endif // PIR_SERVER_H