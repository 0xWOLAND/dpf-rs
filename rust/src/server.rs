use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;
use crate::error::{PirError, PirStatus};

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

pub struct PirServer<T> {
    handle: *mut c_void,
    elements: Vec<T>,
}

impl<T> PirServer<T>
where
    T: AsRef<str> + Clone,
{
    pub fn new(elements: &[T]) -> Result<Self, PirError> {
        if elements.is_empty() {
            return Err(PirError::InvalidArgument);
        }

        unsafe {
            let c_strings: Vec<CString> = elements
                .iter()
                .map(|s| CString::new(s.as_ref()).map_err(|_| PirError::InvalidArgument))
                .collect::<Result<Vec<_>, _>>()?;
            let c_ptrs: Vec<*const c_char> = c_strings
                .iter()
                .map(|cs| cs.as_ptr())
                .collect();
            let mut handle = ptr::null_mut();
            let result: Result<(), PirError> = pir_server_create(
                c_ptrs.as_ptr(), 
                elements.len() as c_int, 
                &mut handle
            ).into();
            
            result.map(|_| Self { 
                handle,
                elements: elements.to_vec(),
            })
        }
    }

    pub fn write(&mut self, element: T) -> Result<(), PirError> {
        self.batch_write(&[element])
    }

    pub fn batch_write(&mut self, new_elements: &[T]) -> Result<(), PirError> {
        let mut updated_elements = self.elements.clone();
        updated_elements.extend(new_elements.iter().cloned());

        let new_server = PirServer::new(&updated_elements)?;
        
        unsafe {
            if !self.handle.is_null() {
                pir_server_destroy(self.handle);
            }
        }
        self.handle = new_server.handle;
        self.elements = updated_elements;
        
        std::mem::forget(new_server);
        
        Ok(())
    }

    pub fn process_request(&self, request_base64: &str) -> Result<String, PirError> {
        unsafe {
            let c_request = CString::new(request_base64)
                .map_err(|_| PirError::InvalidArgument)?;
            
            let mut response_ptr = ptr::null_mut();
            let result: Result<(), PirError> = pir_server_process_request(
                self.handle, 
                c_request.as_ptr(), 
                &mut response_ptr
            ).into();
            result.and_then(|_| {
                if response_ptr.is_null() {
                    return Err(PirError::FfiError);
                }
                let result = CStr::from_ptr(response_ptr)
                    .to_str()
                    .map(String::from)
                    .map_err(|_| PirError::Utf8Error)?;
                pir_server_free_string(response_ptr);
                Ok(result)
            })
        }
    }

    pub fn get_elements(&self) -> &[T] {
        &self.elements
    }
}

impl<T> Drop for PirServer<T> {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                pir_server_destroy(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let elements = vec!["test1", "test2", "test3"];
        let server = PirServer::new(&elements);
        assert!(server.is_ok());
    }

    #[test]
    fn test_empty_elements() {
        let elements: Vec<String> = vec![];
        let server = PirServer::new(&elements);
        assert!(matches!(server, Err(PirError::InvalidArgument)));
    }

    #[test]
    fn test_write() {
        let elements = vec!["test1", "test2"];
        let mut server = PirServer::new(&elements).unwrap();
        assert!(server.write("test3").is_ok());
        assert_eq!(server.get_elements(), &["test1", "test2", "test3"]);
    }

    #[test]
    fn test_batch_write() {
        let elements = vec!["test1"];
        let mut server = PirServer::new(&elements).unwrap();
        assert!(server.batch_write(&["test2", "test3"]).is_ok());
        assert_eq!(server.get_elements(), &["test1", "test2", "test3"]);
    }

    #[test]
    fn test_with_string_type() {
        let elements: Vec<String> = vec!["test1".to_string(), "test2".to_string()];
        let mut server = PirServer::new(&elements).unwrap();
        assert!(server.write("test3".to_string()).is_ok());
        assert_eq!(
            server.get_elements(),
            &[
                "test1".to_string(),
                "test2".to_string(),
                "test3".to_string()
            ]
        );
    }
}