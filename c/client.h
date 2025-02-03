#ifndef PIR_CLIENT_H
#define PIR_CLIENT_H

#ifdef __cplusplus
extern "C" {
#endif

// Opaque pointer to client state
typedef struct PirClientWrapper PirClientWrapper;

// Create a new PIR client instance
PirClientWrapper* pir_client_create(int database_size);

// Generate PIR requests for given indices
// Returns a JSON string containing both requests that must be freed using pir_client_free_string
char* pir_client_generate_requests(PirClientWrapper* client, const int* indices, int num_indices);

// Process responses from both servers
// Returns the merged result that must be freed using pir_client_free_string
char* pir_client_process_responses(const char* serialized_responses);

// Free a string allocated by the client
void pir_client_free_string(char* str);

// Destroy the client instance
void pir_client_destroy(PirClientWrapper* client);

#ifdef __cplusplus
}
#endif

#endif // PIR_CLIENT_H