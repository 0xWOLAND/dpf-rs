use libc::{c_void, size_t, c_char};
use std::ffi::{CStr, CString};
use std::slice;

#[repr(C)]
pub struct DpfPirBuffer {
    data: *mut u8,
    size: size_t,
}

#[repr(C)]
pub struct DpfPirRequest {
    leader_request: DpfPirBuffer,
    helper_request: DpfPirBuffer,
    client_state: DpfPirBuffer,
}

#[repr(C)]
pub struct DpfPirResponse {
    values: *mut *mut c_char,
    lengths: *mut size_t,
    num_values: size_t,
}

#[repr(C)]
pub struct DpfPirConfig {
    database_size: u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DpfPirStatus {
    Ok = 0,
    InvalidArgument = 1,
    FailedPrecondition = 2,
    OutOfMemory = 3,
    InternalError = 4,
}

pub struct DpfPirClient {
    inner: *mut c_void,
}

type DpfPirEncryptRequestFn = extern "C" fn(
    plaintext: *const DpfPirBuffer,
    context_info: *const c_char,
    ciphertext: *mut DpfPirBuffer,
    user_data: *mut c_void,
) -> DpfPirStatus;

extern "C" {
    fn dpf_pir_client_create(
        config: *const DpfPirConfig,
        encrypt_fn: DpfPirEncryptRequestFn,
        user_data: *mut c_void,
        encryption_context_info: *const c_char,
        client: *mut *mut c_void,
    ) -> DpfPirStatus;

    fn dpf_pir_client_destroy(client: *mut c_void);

    fn dpf_pir_client_create_request(
        client: *mut c_void,
        indices: *const i32,
        num_indices: size_t,
        request: *mut DpfPirRequest,
    ) -> DpfPirStatus;

    fn dpf_pir_client_handle_response(
        client: *mut c_void,
        response: *const DpfPirBuffer,
        client_state: *const DpfPirBuffer,
        result: *mut DpfPirResponse,
    ) -> DpfPirStatus;

    fn dpf_pir_request_free(request: *mut DpfPirRequest);
    fn dpf_pir_response_free(response: *mut DpfPirResponse);
    fn dpf_pir_buffer_free(buffer: *mut DpfPirBuffer);
    fn dpf_pir_get_last_error() -> *const c_char;
}

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl DpfPirClient {
    pub fn new(database_size: u64, encryption_context: Option<&str>) -> Result<Self, Error> {
        let config = DpfPirConfig { database_size };
        let mut client = std::ptr::null_mut();
        
        // Convert encryption context to C string if provided
        let context_cstring = encryption_context.map(|s| CString::new(s).unwrap());
        let context_ptr = context_cstring
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null());

        let status = unsafe {
            dpf_pir_client_create(
                &config,
                Self::encrypt_request_callback,
                std::ptr::null_mut(),
                context_ptr,
                &mut client,
            )
        };

        if status != DpfPirStatus::Ok {
            let error = unsafe { CStr::from_ptr(dpf_pir_get_last_error()) }
                .to_string_lossy()
                .into_owned();
            return Err(Error { message: error });
        }

        Ok(Self { inner: client })
    }

    extern "C" fn encrypt_request_callback(
        plaintext: *const DpfPirBuffer,
        _context_info: *const c_char,
        ciphertext: *mut DpfPirBuffer,
        _user_data: *mut c_void,
    ) -> DpfPirStatus {
        // TODO: Implement actual encryption
        // For now, just copy the plaintext
        unsafe {
            let plain = slice::from_raw_parts((*plaintext).data, (*plaintext).size);
            let mut cipher = Vec::from(plain);
            (*ciphertext).data = cipher.as_mut_ptr();
            (*ciphertext).size = cipher.len();
            std::mem::forget(cipher);
        }
        DpfPirStatus::Ok
    }

    pub fn create_request(&self, indices: &[i32]) -> Result<DpfPirRequest, Error> {
        let mut request = DpfPirRequest {
            leader_request: DpfPirBuffer { data: std::ptr::null_mut(), size: 0 },
            helper_request: DpfPirBuffer { data: std::ptr::null_mut(), size: 0 },
            client_state: DpfPirBuffer { data: std::ptr::null_mut(), size: 0 },
        };

        let status = unsafe {
            dpf_pir_client_create_request(
                self.inner,
                indices.as_ptr(),
                indices.len(),
                &mut request,
            )
        };

        if status != DpfPirStatus::Ok {
            let error = unsafe { CStr::from_ptr(dpf_pir_get_last_error()) }
                .to_string_lossy()
                .into_owned();
            return Err(Error { message: error });
        }

        Ok(request)
    }

    pub fn handle_response(
        &self,
        response: &DpfPirBuffer,
        client_state: &DpfPirBuffer,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let mut result = DpfPirResponse {
            values: std::ptr::null_mut(),
            lengths: std::ptr::null_mut(),
            num_values: 0,
        };

        let status = unsafe {
            dpf_pir_client_handle_response(self.inner, response, client_state, &mut result)
        };

        if status != DpfPirStatus::Ok {
            let error = unsafe { CStr::from_ptr(dpf_pir_get_last_error()) }
                .to_string_lossy()
                .into_owned();
            return Err(Error { message: error });
        }

        let values = unsafe {
            let lengths = slice::from_raw_parts(result.lengths, result.num_values);
            let mut values = Vec::with_capacity(result.num_values);
            
            for i in 0..result.num_values {
                let value_ptr = *result.values.add(i);
                let value = slice::from_raw_parts(value_ptr as *const u8, lengths[i]);
                values.push(Vec::from(value));
            }
            
            dpf_pir_response_free(&mut result);
            values
        };

        Ok(values)
    }
}

impl Drop for DpfPirClient {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { dpf_pir_client_destroy(self.inner) };
        }
    }
}

impl Drop for DpfPirRequest {
    fn drop(&mut self) {
        unsafe { dpf_pir_request_free(self) };
    }
}