use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;

use crate::{PirError, PirStatus};

#[link(name = "dpf_server")]
extern "C" {
    fn pir_server_create(
        elements: *const *const c_char,
        num_elements: c_int,
        server_handle: *mut *mut c_void,
    ) -> PirStatus;

    fn pir_server_process_request(
        server_handle: *mut c_void,
        request_base64: *const c_char,
        response_base64: *mut *mut c_char,
    ) -> PirStatus;

    fn pir_server_free_string(str: *mut c_char);
    fn pir_server_destroy(server_handle: *mut c_void);
}

pub struct PirServer {
    handle: *mut c_void,
}

impl PirServer {
    /// Create a new server with actual data elements
    pub fn new<T: AsRef<str>>(elements: &[T]) -> Result<Self, PirError> {
        unsafe {
            // Create vector of CStrings first to keep them alive
            let c_strings: Vec<CString> = elements
                .iter()
                .map(|s| CString::new(s.as_ref()).map_err(|_| PirError::InvalidArgument))
                .collect::<Result<Vec<_>, _>>()?;
    
            // Then create vector of pointers to those CStrings
            let c_elements: Vec<*const c_char> = c_strings
                .iter()
                .map(|cs| cs.as_ptr())
                .collect();
    
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

    pub fn process_request(&self, request_base64: &str) -> Result<String, PirError> {
        unsafe {
            let c_request = CString::new(request_base64)
                // .map_err(|e| PirError::InvalidArgument(e.to_string()))?;
                .map_err(|e| PirError::InvalidArgument)?;
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
        Ok(c_str.to_str().unwrap().to_owned())
    }
}
