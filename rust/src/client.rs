use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;

use crate::{PirError, PirStatus};

#[link(name = "dpf_client")]
extern "C" {
    fn pir_client_create(database_size: c_int, client_handle: *mut *mut c_void) -> PirStatus;

    fn pir_client_generate_requests(
        client_handle: *mut c_void,
        indices: *const c_int,
        num_indices: c_int,
        requests_json: *mut *mut c_char,
    ) -> PirStatus;

    fn pir_client_process_responses(
        responses_json: *const c_char,
        merged_result: *mut *mut c_char,
    ) -> PirStatus;

    fn pir_client_free_string(str: *mut c_char);
    fn pir_client_destroy(client_handle: *mut c_void);
}

pub struct PirClient {
    handle: *mut c_void,
}

impl PirClient {
    pub fn new(database_size: i32) -> Result<Self, PirError> {
        unsafe {
            let mut handle = ptr::null_mut();
            match pir_client_create(database_size, &mut handle) {
                PirStatus::Success => Ok(PirClient { handle }),
                status => Err(get_error_with_status(status)),
            }
        }
    }

    pub fn generate_requests(&self, indices: &[i32]) -> Result<String, PirError> {
        unsafe {
            let mut requests_json = ptr::null_mut();
            let status = pir_client_generate_requests(
                self.handle,
                indices.as_ptr(),
                indices.len() as c_int,
                &mut requests_json,
            );

            match status {
                PirStatus::Success => {
                    let result = c_char_to_string(requests_json)?;
                    pir_client_free_string(requests_json);
                    Ok(result)
                }
                status => Err(get_error_with_status(status)),
            }
        }
    }

    pub fn process_responses(&self, responses_json: &str) -> Result<String, PirError> {
        unsafe {
            let c_responses = CString::new(responses_json)
                // .map_err(|e| PirError::InvalidArgument(e.to_string()))?;
                .map_err(|e| PirError::InvalidArgument)?;
            let mut merged_result = ptr::null_mut();

            let status = pir_client_process_responses(c_responses.as_ptr(), &mut merged_result);

            match status {
                PirStatus::Success => {
                    let result = c_char_to_string(merged_result)?;
                    pir_client_free_string(merged_result);
                    Ok(result)
                }
                status => Err(get_error_with_status(status)),
            }
        }
    }
}

impl Drop for PirClient {
    fn drop(&mut self) {
        unsafe {
            pir_client_destroy(self.handle);
        }
    }
}


fn get_error_with_status(status: PirStatus) -> PirError {
    match status {
        PirStatus::ErrorInvalidArgument => PirError::InvalidArgument,
        PirStatus::ErrorMemory => PirError::Memory,
        PirStatus::ErrorProcessing => PirError::Processing,
        _ => PirError::FfiError,
    }
}

fn c_char_to_string(ptr: *mut c_char) -> Result<String, PirError> {
    if ptr.is_null() {
        return Err(PirError::FfiError);
    }
    unsafe {
        let c_str = CStr::from_ptr(ptr);
        c_str.to_str()
            .map_err(|e| PirError::FfiError)
            .map(|s| s.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print() {
        eprintln!("Hello, world!");
    }

    #[test]
    fn test_pir_client_lifecycle() -> Result<(), PirError> {
        let client = PirClient::new(100)?;
        let indices = vec![1, 2, 3];

        let requests_json = client.generate_requests(&indices)?;
        assert!(!requests_json.is_empty());

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        assert!(matches!(
            PirClient::new(-1),
            Err(PirError::InvalidArgument)
        ));

        let client = PirClient::new(100).unwrap();

        assert!(matches!(
            client.generate_requests(&[]),
            Err(PirError::InvalidArgument)
        ));

        assert!(matches!(
            client.process_responses("invalid json"),
            Err(PirError::Processing)
        ));
    }
}
