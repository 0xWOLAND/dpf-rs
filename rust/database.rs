use libc::{c_char, c_void, size_t};
use std::error::Error;
use std::ffi::CStr;
use std::fmt;

#[repr(C)]
pub struct DpfPirBuffer {
    data: *mut u8,
    size: size_t,
}

#[repr(C)]
pub struct DpfPirConfig {
    database_size: u64,
}

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

#[link(name = "dpf_database")]
extern "C" {
    fn dpf_pir_builder_create(builder: *mut *mut c_void) -> DpfPirStatus;
    fn dpf_pir_builder_destroy(builder: *mut c_void);
    fn dpf_pir_builder_insert(
        builder: *mut c_void,
        value: *const u8,
        value_length: size_t,
    ) -> DpfPirStatus;
    fn dpf_pir_builder_clear(builder: *mut c_void) -> DpfPirStatus;
    fn dpf_pir_builder_build(
        builder: *mut c_void,
        database: *mut *mut c_void,
    ) -> DpfPirStatus;
    fn dpf_pir_database_destroy(database: *mut c_void);
    fn dpf_pir_get_last_error() -> *const c_char;
    fn dpf_pir_buffer_free(buffer: *mut DpfPirBuffer);
}

pub struct DatabaseBuilder {
    inner: *mut c_void,
}

impl DatabaseBuilder {
    pub fn new() -> Result<Self, DpfError> {
        let mut builder = std::ptr::null_mut();
        unsafe {
            match dpf_pir_builder_create(&mut builder) {
                DpfPirStatus::Ok => Ok(DatabaseBuilder { inner: builder }),
                _ => {
                    let error = CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned();
                    Err(DpfError { message: error })
                }
            }
        }
    }

    pub fn insert(&mut self, value: &[u8]) -> Result<(), DpfError> {
        unsafe {
            match dpf_pir_builder_insert(self.inner, value.as_ptr(), value.len()) {
                DpfPirStatus::Ok => Ok(()),
                _ => {
                    let error = CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned();
                    Err(DpfError { message: error })
                }
            }
        }
    }

    pub fn clear(&mut self) -> Result<(), DpfError> {
        unsafe {
            match dpf_pir_builder_clear(self.inner) {
                DpfPirStatus::Ok => Ok(()),
                _ => {
                    let error = CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned();
                    Err(DpfError { message: error })
                }
            }
        }
    }

    pub fn build(self) -> Result<Database, DpfError> {
        let mut database = std::ptr::null_mut();
        unsafe {
            match dpf_pir_builder_build(self.inner, &mut database) {
                DpfPirStatus::Ok => {
                    // Don't call destroy on builder since build takes ownership
                    std::mem::forget(self);
                    Ok(Database { inner: database })
                }
                _ => {
                    let error = CStr::from_ptr(dpf_pir_get_last_error())
                        .to_string_lossy()
                        .into_owned();
                    Err(DpfError { message: error })
                }
            }
        }
    }
}

impl Drop for DatabaseBuilder {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                dpf_pir_builder_destroy(self.inner);
                self.inner = std::ptr::null_mut();
            }
        }
    }
}

pub struct Database {
    inner: *mut c_void,
}

impl Database {
    pub fn builder() -> Result<DatabaseBuilder, DpfError> {
        DatabaseBuilder::new()
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        unsafe {
            if !self.inner.is_null() {
                dpf_pir_database_destroy(self.inner);
                self.inner = std::ptr::null_mut();
            }
        }
    }
}

// Safety: Database can be sent between threads safely
unsafe impl Send for Database {}
// Safety: Database can be shared between threads safely
unsafe impl Sync for Database {}