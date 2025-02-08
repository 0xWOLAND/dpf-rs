
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

#[cfg(test)]
mod tests {
    use super::*;

    const N: usize = 100;

    #[test]
    fn test_e2e() {
        let test_elements = (0..N).map(|i| format!("Element{}", i)).collect::<Vec<_>>();

        let client = client::PirClient::new(N as i32).unwrap();
        let server1 = server::PirServer::new(&test_elements).unwrap();
        let server2 = server::PirServer::new(&test_elements).unwrap();

        let indices = vec![1, 2, 3];
        let requests_json = client.generate_requests(&indices).unwrap();
        
    }
}
