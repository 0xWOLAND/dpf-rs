use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::error::{PirError, PirStatus};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub request1: String,
    pub request2: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub response1: String,
    pub response2: String,
}

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

pub struct Client {
    handle: *mut c_void,
}

impl Client {
    pub fn new(database_size: i32) -> Result<Self, PirError> {
        if database_size <= 0 {
            return Err(PirError::InvalidArgument);
        }

        unsafe {
            let mut handle = ptr::null_mut();
            let result: Result<(), PirError> = pir_client_create(database_size, &mut handle).into();
            result.map(|_| Self { handle })
        }
    }

    pub fn update_size(&mut self, new_size: i32) -> Result<(), PirError> {
        if new_size <= 0 {
            return Err(PirError::InvalidArgument);
        }

        let new_client = Client::new(new_size)?;
        
        unsafe {
            if !self.handle.is_null() {
                pir_client_destroy(self.handle);
            }
            self.handle = new_client.handle;
            std::mem::forget(new_client);
        }
        
        Ok(())
    }

    pub fn generate_requests(&self, indices: &[i32]) -> Result<Request, PirError> {
        unsafe {
            let mut requests_json = ptr::null_mut();
            let result: Result<(), PirError> = pir_client_generate_requests(
                self.handle,
                indices.as_ptr(),
                indices.len() as c_int,
                &mut requests_json,
            ).into();

            result.and_then(|_| {
                if requests_json.is_null() {
                    return Err(PirError::FfiError);
                }
                let result = CStr::from_ptr(requests_json)
                    .to_str()
                    .map(String::from)
                    .map_err(|_| PirError::Utf8Error)?;
                pir_client_free_string(requests_json);
                serde_json::from_str(&result).map_err(|_| PirError::Processing)
            })
        }
    }

    pub fn _process_responses(&self, response: Response) -> Result<String, PirError> {
        unsafe {
            let responses_json = serde_json::to_string(&response)
                .map_err(|_| PirError::Processing)?;
            let c_responses = CString::new(responses_json)
                .map_err(|_| PirError::InvalidArgument)?;
            let mut merged_result = ptr::null_mut();
            let result: Result<(), PirError> = pir_client_process_responses(
                c_responses.as_ptr(), 
                &mut merged_result
            ).into();
            result.and_then(|_| {
                if merged_result.is_null() {
                    return Err(PirError::FfiError);
                }
                let result = CStr::from_ptr(merged_result)
                    .to_str()
                    .map(String::from)
                    .map_err(|_| PirError::Utf8Error)?;
                pir_client_free_string(merged_result);
                Ok(result)
            })
        }
    }

    pub fn process_responses(&self, response: Response) -> Result<Vec<u8>, PirError> {
        self._process_responses(response)
            .map(|result| BASE64.decode(result).unwrap())
            .map_err(|_| PirError::Processing)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                pir_client_destroy(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pir_client_lifecycle() -> Result<(), PirError> {
        let client = Client::new(100)?;
        let indices = vec![1, 2, 3];
        let requests = client.generate_requests(&indices)?;
        assert!(!requests.request1.is_empty());
        assert!(!requests.request2.is_empty());
        Ok(())
    }

    #[test]
    fn test_update_size() -> Result<(), PirError> {
        let mut client = Client::new(100)?;
        
        assert!(client.update_size(200).is_ok());
        
        let indices = vec![150];
        let requests = client.generate_requests(&indices)?;
        assert!(!requests.request1.is_empty());
        assert!(!requests.request2.is_empty());
        
        assert!(matches!(
            client.update_size(-1),
            Err(PirError::InvalidArgument)
        ));
        
        Ok(())
    }

    #[test]
    fn test_error_handling() {
        assert!(matches!(
            Client::new(-1),
            Err(PirError::InvalidArgument)
        ));
    }
}