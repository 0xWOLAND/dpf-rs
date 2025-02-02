#ifndef DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_DATABASE_C_H_
#define DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_DATABASE_C_H_

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque types
typedef struct DpfPirDatabase_st* DpfPirDatabase;
typedef struct DpfPirDatabaseBuilder_st* DpfPirDatabaseBuilder;

// Status codes
typedef enum {
    DPF_PIR_OK = 0,
    DPF_PIR_INVALID_ARGUMENT = 1,
    DPF_PIR_FAILED_PRECONDITION = 2,
    DPF_PIR_OUT_OF_MEMORY = 3,
    DPF_PIR_INTERNAL_ERROR = 4
} DpfPirStatus;

// Buffer structure for data transfer
typedef struct {
    uint8_t* data;
    size_t size;
} DpfPirBuffer;

// Database builder functions
DpfPirStatus dpf_pir_builder_create(DpfPirDatabaseBuilder* builder);
void dpf_pir_builder_destroy(DpfPirDatabaseBuilder builder);
DpfPirStatus dpf_pir_builder_insert(DpfPirDatabaseBuilder builder, 
                                   const uint8_t* value, 
                                   size_t value_length);
DpfPirStatus dpf_pir_builder_clear(DpfPirDatabaseBuilder builder);
DpfPirStatus dpf_pir_builder_build(DpfPirDatabaseBuilder builder,
                                  DpfPirDatabase* database);

// Database functions
DpfPirStatus dpf_pir_database_size(const DpfPirDatabase database, 
                                  size_t* size);
DpfPirStatus dpf_pir_database_selection_bits(const DpfPirDatabase database, 
                                           size_t* num_bits);
DpfPirStatus dpf_pir_database_inner_product(const DpfPirDatabase database,
                                           const DpfPirBuffer* selections,
                                           size_t num_selections,
                                           DpfPirBuffer* result);

void dpf_pir_database_destroy(DpfPirDatabase database);
void dpf_pir_buffer_free(DpfPirBuffer* buffer);

// Error handling
const char* dpf_pir_get_last_error(void);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // DISTRIBUTED_POINT_FUNCTIONS_PIR_DENSE_DPF_PIR_DATABASE_C_H_