use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;

use crate::{PirStatus, PirError};

#[link(name = "dpf_client", kind = "static")]
extern "C" {
    fn pir_client_initialize() -> PirStatus;
    fn pir_client_cleanup();
    
    fn pir_client_create(
        database_size: c_int,
        client_handle: *mut *mut c_void,
    ) -> PirStatus;

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
    fn pir_client_get_last_error() -> *const c_char;
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
                .map_err(|e| PirError::InvalidArgument(e.to_string()))?;
            let mut merged_result = ptr::null_mut();

            let status = pir_client_process_responses(
                c_responses.as_ptr(),
                &mut merged_result,
            );

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
            pir_client_cleanup();
        }
    }
}

pub fn initialize() -> Result<(), PirError> {
    unsafe {
        match pir_client_initialize() {
            PirStatus::Success => Ok(()),
            status => Err(get_error_with_status(status)),
        }
    }
}

fn get_error_with_status(status: PirStatus) -> PirError {
    let error_msg = unsafe {
        let error_ptr = pir_client_get_last_error();
        if error_ptr.is_null() {
            "Unknown error".to_string()
        } else {
            CStr::from_ptr(error_ptr)
                .to_string_lossy()
                .into_owned()
        }
    };

    match status {
        PirStatus::ErrorInvalidArgument => PirError::InvalidArgument(error_msg),
        PirStatus::ErrorMemory => PirError::Memory(error_msg),
        PirStatus::ErrorProcessing => PirError::Processing(error_msg),
        _ => PirError::FfiError(error_msg),
    }
}

fn c_char_to_string(ptr: *mut c_char) -> Result<String, PirError> {
    if ptr.is_null() {
        return Err(PirError::FfiError("Null pointer received".to_string()));
    }
    unsafe {
        let c_str = CStr::from_ptr(ptr);
        Ok(c_str.to_str()?.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pir_client_lifecycle() -> Result<(), PirError> {
        initialize()?;

        let client = PirClient::new(100)?;
        let indices = vec![1, 2, 3];
        
        let requests_json = client.generate_requests(&indices)?;
        assert!(!requests_json.is_empty());

        let mock_responses = r#"{"response1": "base64data1", "response2": "base64data2"}"#;
        let result = client.process_responses(mock_responses)?;
        assert!(!result.is_empty());

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        initialize().unwrap();
        
        assert!(matches!(
            PirClient::new(-1),
            Err(PirError::InvalidArgument(_))
        ));

        let client = PirClient::new(100).unwrap();

        assert!(matches!(
            client.generate_requests(&[]),
            Err(PirError::InvalidArgument(_))
        ));

        assert!(matches!(
            client.process_responses("invalid json"),
            Err(PirError::Processing(_))
        ));
    }
}