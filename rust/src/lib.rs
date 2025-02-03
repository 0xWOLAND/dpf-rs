
pub mod client;
pub mod server;

use thiserror::Error;

#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum PirStatus {
    Success = 0,
    ErrorInvalidArgument = -1,
    ErrorMemory = -2,
    ErrorProcessing = -3,
}

#[derive(Error, Debug)]
pub enum PirError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Memory error: {0}")]
    Memory(String),
    #[error("Processing error: {0}")]
    Processing(String),
    #[error("Invalid UTF-8 in response: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("FFI error: {0}")]
    FfiError(String),
}
