
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
    #[error("Invalid argument")]
    InvalidArgument,
    #[error("Memory error")]
    Memory,
    #[error("Processing error")]
    Processing,
    #[error("Invalid UTF-8 in response")]
    Utf8Error,
    #[error("FFI error")]
    FfiError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::*;
    use server::*;

    const N: usize = 4;

    // #[test]
    // fn test_e2e() {
    //     let test_elements = (0..N).map(|i| format!("Element{}", i)).collect::<Vec<_>>();

    //     let client = PirClient::new(N as i32).unwrap();
    //     let server1 = PirServer::new(&test_elements).unwrap();
    //     let server2 = PirServer::new(&test_elements).unwrap();

    //     let indices = vec![0, 2];
    //     let Request { request1, request2 } = client.generate_requests(&indices).unwrap();

    //     println!("[RUST] Request1: {}", request1);
    //     println!("[RUST] Request2: {}", request2);

    //     let response1 = server1.process_request(&request1).unwrap();
    //     let response2 = server2.process_request(&request2).unwrap();

    //     let response = client.process_responses(Response { response1, response2} ).unwrap();
    //     println!("[RUST] Response: {}", response);

    // }
    #[test]
    fn test_e2e() {
        // Match C++ setup exactly
        let test_elements = vec![
            "Element0".to_string(),
            "Element1".to_string(), 
            "Element2".to_string(),
            "Element3".to_string()
        ];
        
        let client = PirClient::new(N as i32).unwrap();
        let server1 = PirServer::new(&test_elements).unwrap();
        let server2 = PirServer::new(&test_elements).unwrap();
        
        // Match C++ test indices
        let indices = vec![0, 2];  
        
        let Request { request1, request2 } = client.generate_requests(&indices).unwrap();
        println!("[RUST] Request1: {}", request1);
        println!("[RUST] Request2: {}", request2);
        
        let response1 = server1.process_request(&request1).unwrap();
        let response2 = server2.process_request(&request2).unwrap();
        
        // Debug print the responses before processing
        println!("[RUST] Raw response1: {}", response1);
        println!("[RUST] Raw response2: {}", response2);
        
        let response = client.process_responses(Response { response1, response2 }).unwrap();
        println!("[RUST] Final response: {}", response);
    }
}
