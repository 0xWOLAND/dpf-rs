use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::os::raw::{c_char, c_void};
use libc::size_t;

#[repr(C)]
pub struct DpfPirBuffer {
    data: *mut u8,
    size: size_t,
}

impl DpfPirBuffer {
    pub(crate) fn from_vec(mut vec: Vec<u8>) -> Self {
        let size = vec.len();
        let data = vec.as_mut_ptr();
        std::mem::forget(vec); // Don't drop the vector
        DpfPirBuffer { data, size }
    }

    pub(crate) unsafe fn to_vec(&self) -> Vec<u8> {
        Vec::from_raw_parts(self.data, self.size, self.size)
    }
}

#[repr(C)]
pub struct DpfPirRequest {
    leader_request: DpfPirBuffer,
    helper_request: DpfPirBuffer,
}

#[repr(C)]
pub struct DpfPirConfig {
    database_size: u64,
}

pub type ForwardHelperRequestFn = Box<dyn Fn(&[u8], &str) -> Result<Vec<u8>, DpfError> + Send + Sync>;
pub type DecryptHelperRequestFn = Box<dyn Fn(&[u8], &str) -> Result<Vec<u8>, DpfError> + Send + Sync>;

#[derive(Debug)]
pub struct DpfError {
    message: String,
}

impl fmt::Display for DpfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DPF error: {}", self.message)
    }
}

impl Error for DpfError {}

#[repr(C)]
#[derive(Debug)]
pub enum DpfPirStatus {
    Ok = 0,
    InvalidArgument = 1,
    FailedPrecondition = 2,
    OutOfMemory = 3,
    InternalError = 4,
}

// Extern C functions
#[link(name = "dpf_server")]
extern "C" {
    fn dpf_pir_server_create_leader(
        config: *const DpfPirConfig,
        database: *mut c_void,
        forward_fn: unsafe extern "C" fn(*const DpfPirBuffer, *const c_char, *mut DpfPirBuffer, *mut c_void) -> DpfPirStatus,
        user_data: *mut c_void,
        server: *mut *mut c_void,
    ) -> DpfPirStatus;

    fn dpf_pir_server_create_helper(
        config: *const DpfPirConfig,
        database: *mut c_void,
        decrypt_fn: unsafe extern "C" fn(*const DpfPirBuffer, *const c_char, *mut DpfPirBuffer, *mut c_void) -> DpfPirStatus,
        user_data: *mut c_void,
        server: *mut *mut c_void,
    ) -> DpfPirStatus;

    fn dpf_pir_server_create_plain(
        config: *const DpfPirConfig,
        database: *mut c_void,
        server: *mut *mut c_void,
    ) -> DpfPirStatus;

    fn dpf_pir_server_handle_request(
        server: *mut c_void,
        request: *const DpfPirRequest,
        response: *mut DpfPirBuffer,
    ) -> DpfPirStatus;

    fn dpf_pir_server_destroy(server: *mut c_void);
    fn dpf_pir_get_last_error() -> *const c_char;
}

pub struct Server {
    inner: *mut c_void,
    callbacks: ServerCallbacks,
}

enum ServerCallbacks {
    None,
    Forward(ForwardHelperRequestFn),
    Decrypt(DecryptHelperRequestFn),
}

impl Server {
    pub fn new_leader(
        config: &DpfPirConfig,
        database: *mut c_void,
        forward_fn: impl Fn(&[u8], &str) -> Result<Vec<u8>, DpfError> + Send + Sync + Clone + 'static,
    ) -> Result<Self, DpfError> {
        let mut server = std::ptr::null_mut();
        let boxed_fn = Box::new(Box::new(forward_fn.clone()) as ForwardHelperRequestFn);
        let user_data = Box::into_raw(boxed_fn) as *mut c_void;

        unsafe extern "C" fn forward_trampoline(
            request: *const DpfPirBuffer,
            context_info: *const c_char,
            response: *mut DpfPirBuffer,
            user_data: *mut c_void,
        ) -> DpfPirStatus {
            unsafe {
                let callback = &*(user_data as *const ForwardHelperRequestFn);
                let context = CStr::from_ptr(context_info).to_string_lossy();
                let request_data = std::slice::from_raw_parts((*request).data, (*request).size);
                
                match callback(request_data, &context) {
                    Ok(resp_vec) => {
                        *response = DpfPirBuffer::from_vec(resp_vec);
                        DpfPirStatus::Ok
                    }
                    Err(_) => DpfPirStatus::InternalError,
                }
            }
        }

        let status = unsafe {
            dpf_pir_server_create_leader(
                config,
                database,
                forward_trampoline,
                user_data,
                &mut server,
            )
        };

        match status {
            DpfPirStatus::Ok => Ok(Server {
                inner: server,
                callbacks: ServerCallbacks::Forward(Box::new(forward_fn)),
            }),
            _ => {
                unsafe {
                    let _ = Box::from_raw(user_data as *mut ForwardHelperRequestFn);
                }
                let error = unsafe {
                    CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned()
                };
                Err(DpfError { message: error })
            }
        }
    }

    pub fn new_helper(
        config: &DpfPirConfig,
        database: *mut c_void,
        decrypt_fn: impl Fn(&[u8], &str) -> Result<Vec<u8>, DpfError> + Send + Sync + Clone + 'static,
    ) -> Result<Self, DpfError> {
        let mut server = std::ptr::null_mut();
        let boxed_fn = Box::new(Box::new(decrypt_fn.clone()) as DecryptHelperRequestFn);
        let user_data = Box::into_raw(boxed_fn) as *mut c_void;

        unsafe extern "C" fn decrypt_trampoline(
            ciphertext: *const DpfPirBuffer,
            context_info: *const c_char,
            plaintext: *mut DpfPirBuffer,
            user_data: *mut c_void,
        ) -> DpfPirStatus {
            unsafe {
                let callback = &*(user_data as *const DecryptHelperRequestFn);
                let context = CStr::from_ptr(context_info).to_string_lossy();
                let cipher_data = std::slice::from_raw_parts((*ciphertext).data, (*ciphertext).size);
                
                match callback(cipher_data, &context) {
                    Ok(plain_vec) => {
                        *plaintext = DpfPirBuffer::from_vec(plain_vec);
                        DpfPirStatus::Ok
                    }
                    Err(_) => DpfPirStatus::InternalError,
                }
            }
        }

        let status = unsafe {
            dpf_pir_server_create_helper(
                config,
                database,
                decrypt_trampoline,
                user_data,
                &mut server,
            )
        };

        match status {
            DpfPirStatus::Ok => Ok(Server {
                inner: server,
                callbacks: ServerCallbacks::Decrypt(Box::new(decrypt_fn)),
            }),
            _ => {
                unsafe {
                    let _ = Box::from_raw(user_data as *mut DecryptHelperRequestFn);
                }
                let error = unsafe {
                    CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned()
                };
                Err(DpfError { message: error })
            }
        }
    }

    pub fn new_plain(config: &DpfPirConfig, database: *mut c_void) -> Result<Self, DpfError> {
        let mut server = std::ptr::null_mut();

        let status = unsafe {
            dpf_pir_server_create_plain(config, database, &mut server)
        };

        match status {
            DpfPirStatus::Ok => Ok(Server {
                inner: server,
                callbacks: ServerCallbacks::None,
            }),
            _ => {
                let error = unsafe {
                    CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned()
                };
                Err(DpfError { message: error })
            }
        }
    }

    pub fn handle_request(&self, request: &DpfPirRequest) -> Result<Vec<u8>, DpfError> {
        let mut response = DpfPirBuffer {
            data: std::ptr::null_mut(),
            size: 0,
        };

        let status = unsafe {
            dpf_pir_server_handle_request(self.inner, request, &mut response)
        };

        match status {
            DpfPirStatus::Ok => unsafe {
                Ok(response.to_vec())
            },
            _ => {
                let error = unsafe {
                    CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned()
                };
                Err(DpfError { message: error })
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                dpf_pir_server_destroy(self.inner);
                self.inner = std::ptr::null_mut();
            }
        }
    }
}

// Safety: Server can be sent between threads safely
unsafe impl Send for Server {}
// Safety: Server can be shared between threads safely
unsafe impl Sync for Server {}