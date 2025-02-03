#ifndef PIR_STATUS_H
#define PIR_STATUS_H

#ifdef __cplusplus
extern "C" {
#endif

// Status codes for PIR operations (matching server codes)
typedef enum {
    PIR_SUCCESS = 0,
    PIR_ERROR_INVALID_ARGUMENT = -1,
    PIR_ERROR_MEMORY = -2,
    PIR_ERROR_PROCESSING = -3
} pir_status_t;

#ifdef __cplusplus
}
#endif

#endif // PIR_STATUS_H