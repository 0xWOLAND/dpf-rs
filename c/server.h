#pragma once

#include <memory>
#include <string>
#include <vector>

#include "google/protobuf/arena.h"
#include "pir/dense_dpf_pir_server.h"

// Add these macro definitions
#define DPF_RETURN_IF_ERROR(expr) \
  do { \
    auto _status = (expr); \
    if (!_status.ok()) return _status; \
  } while (0)

#define DPF_ASSIGN_OR_RETURN(lhs, rexpr) \
  auto _status_or = (rexpr); \
  if (!_status_or.ok()) return _status_or.status(); \
  lhs = std::move(_status_or).value()

// Forward declaration of database handle
typedef struct DpfPirDatabase_st* DpfPirDatabase;

// Opaque handle to the server
typedef struct DpfPirServer_st* DpfPirServer;

// Status codes (shared with client)
typedef enum {
    DPF_PIR_OK = 0,
    DPF_PIR_INVALID_ARGUMENT = 1,
    DPF_PIR_FAILED_PRECONDITION = 2,
    DPF_PIR_OUT_OF_MEMORY = 3,
    DPF_PIR_INTERNAL_ERROR = 4
} DpfPirStatus;

// Buffer structure (shared with client)
typedef struct {
    uint8_t* data;
    size_t size;
} DpfPirBuffer;

// Server roles
typedef enum {
    DPF_PIR_SERVER_LEADER = 0,
    DPF_PIR_SERVER_HELPER = 1
} DpfPirServerRole;

// Callback function types
typedef DpfPirStatus (*DpfPirForwardRequestFn)(const DpfPirBuffer* request, void* user_data);
typedef DpfPirStatus (*DpfPirDecryptRequestFn)(const DpfPirBuffer* request, DpfPirBuffer* decrypted, void* user_data);

// Server configuration
typedef struct {
    DpfPirServerRole role;
    DpfPirDatabase database;
    union {
        DpfPirForwardRequestFn forward_fn;
        DpfPirDecryptRequestFn decrypt_fn;
    } callbacks;
    void* callback_user_data;
} DpfPirServerConfig;

// Creates a new DPF PIR server
DpfPirStatus dpf_pir_server_create(
    const DpfPirServerConfig* config,
    DpfPirServer* server);

// Destroys a DPF PIR server
void dpf_pir_server_destroy(DpfPirServer server);

// Gets public parameters for the server
DpfPirStatus dpf_pir_server_get_public_params(
    DpfPirServer server,
    DpfPirBuffer* params);

// Handles a request from a client
DpfPirStatus dpf_pir_server_handle_request(
    DpfPirServer server,
    const DpfPirBuffer* request,
    DpfPirBuffer* response);

// Frees a buffer (shared with client)
void dpf_pir_buffer_free(DpfPirBuffer* buffer);

// Gets the last error message (shared with client)
const char* dpf_pir_get_last_error(void);