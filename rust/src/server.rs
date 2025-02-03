use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;

use crate::{PirError, PirStatus};

extern "C" {
    fn pir_initialize() -> PirStatus;
    fn pir_server_cleanup();
    
    fn pir_server_create(
        elements: *const *const c_char,
        num_elements: c_int,
        server_handle: *mut *mut c_void,
    ) -> PirStatus;

    fn pir_server_create_test(
        database_size: c_int,
        server_handle: *mut *mut c_void,
    ) -> PirStatus;

    fn pir_server_process_request(
        server_handle: *mut c_void,
        request_base64: *const c_char,
        response_base64: *mut *mut c_char,
    ) -> PirStatus;

    fn pir_server_free_string(str: *mut c_char);
    fn pir_server_destroy(server_handle: *mut c_void);
    fn pir_get_last_error() -> *const c_char;
}

pub struct PirServer {
    handle: *mut c_void,
}

impl PirServer {
    /// Create a new server with actual data elements
    pub fn new<T: AsRef<str>>(elements: &[T]) -> Result<Self, PirError> {
        unsafe {
            let c_elements: Vec<*const c_char> = elements
                .iter()
                .map(|s| {
                    CString::new(s.as_ref())
                        .map_err(|e| PirError::InvalidArgument(e.to_string()))
                        .map(|cs| cs.as_ptr())
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut handle = ptr::null_mut();
            match pir_server_create(
                c_elements.as_ptr(),
                elements.len() as c_int,
                &mut handle,
            ) {
                PirStatus::Success => Ok(PirServer { handle }),
                status => Err(get_error_with_status(status)),
            }
        }
    }

    /// Create a new server with test data
    pub fn new_test(database_size: i32) -> Result<Self, PirError> {
        unsafe {
            let mut handle = ptr::null_mut();
            match pir_server_create_test(database_size, &mut handle) {
                PirStatus::Success => Ok(PirServer { handle }),
                status => Err(get_error_with_status(status)),
            }
        }
    }

    pub fn process_request(&self, request_base64: &str) -> Result<String, PirError> {
        unsafe {
            let c_request = CString::new(request_base64)
                .map_err(|e| PirError::InvalidArgument(e.to_string()))?;
            let mut response = ptr::null_mut();

            let status = pir_server_process_request(
                self.handle,
                c_request.as_ptr(),
                &mut response,
            );

            match status {
                PirStatus::Success => {
                    let result = c_char_to_string(response)?;
                    pir_server_free_string(response);
                    Ok(result)
                }
                status => Err(get_error_with_status(status)),
            }
        }
    }
}

impl Drop for PirServer {
    fn drop(&mut self) {
        unsafe {
            pir_server_destroy(self.handle);
            pir_server_cleanup();
        }
    }
}

pub fn initialize() -> Result<(), PirError> {
    unsafe {
        match pir_initialize() {
            PirStatus::Success => Ok(()),
            status => Err(get_error_with_status(status)),
        }
    }
}


fn get_error_with_status(status: PirStatus) -> PirError {
    let error_msg = unsafe {
        let error_ptr = pir_get_last_error();
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
    fn test_server_lifecycle() -> Result<(), PirError> {
        initialize()?;

        // Test with actual data
        let elements = vec!["Element0", "Element1", "Element2"];
        let server = PirServer::new(&elements)?;

        // Test request processing
        let mock_request = "base64encodedrequest";
        let response = server.process_request(mock_request)?;
        assert!(!response.is_empty());

        Ok(())
    }

    #[test]
    fn test_server_test_data() -> Result<(), PirError> {
        initialize()?;

        // Create server with test data
        let server = PirServer::new_test(100)?;

        // Test request processing
        let mock_request = "base64encodedrequest";
        let response = server.process_request(mock_request)?;
        assert!(!response.is_empty());

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        initialize().unwrap();
        
        // Test invalid elements
        let empty_elements: Vec<String> = vec![];
        assert!(matches!(
            PirServer::new(&empty_elements),
            Err(PirError::InvalidArgument(_))
        ));

        // Test with valid server
        let server = PirServer::new_test(100).unwrap();

        // Test invalid request
        assert!(matches!(
            server.process_request(""),
            Err(PirError::InvalidArgument(_))
        ));
    }
}