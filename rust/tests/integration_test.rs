use dpf_rs::{
    client::{PirClient, Request, Response},
    server::PirServer,
    PirError,
};

#[test]
fn test_single_element_query() -> Result<(), PirError> {
    let elements: Vec<String> = (0..4).map(|i| format!("Element{}", i)).collect();
    
    let client = PirClient::new(elements.len() as i32)?;
    let server1 = PirServer::new(&elements)?;
    let server2 = PirServer::new(&elements)?;
    
    // Query for "Element1"
    let indices = vec![1];
    let Request { request1, request2 } = client.generate_requests(&indices)?;
    
    assert!(!request1.is_empty());
    assert!(!request2.is_empty());
    
    let response1 = server1.process_request(&request1)?;
    let response2 = server2.process_request(&request2)?;
    
    assert!(!response1.is_empty());
    assert!(!response2.is_empty());
    
    let final_response = client.process_responses(Response { 
        response1, 
        response2 
    })?;
    
    assert_eq!(final_response, "Element1");
    Ok(())
}

#[test]
fn test_multi_element_query() -> Result<(), PirError> {
    let elements: Vec<String> = (0..4).map(|i| format!("Element{}", i)).collect();
    
    let client = PirClient::new(elements.len() as i32)?;
    let server1 = PirServer::new(&elements)?;
    let server2 = PirServer::new(&elements)?;
    
    let indices = vec![0, 2];
    let Request { request1, request2 } = client.generate_requests(&indices)?;
    
    assert!(!request1.is_empty());
    assert!(!request2.is_empty());
    
    let response1 = server1.process_request(&request1)?;
    let response2 = server2.process_request(&request2)?;
    
    assert!(!response1.is_empty());
    assert!(!response2.is_empty());
    
    let final_response = client.process_responses(Response { 
        response1, 
        response2 
    })?;
    
    assert_eq!(final_response, "Element0, Element2");
    Ok(())
}

#[test]
fn test_generated_data_query() -> Result<(), PirError> {
    const DB_SIZE: usize = 100;
    let elements: Vec<String> = (0..DB_SIZE).map(|i| format!("Element{}", i)).collect();
    
    let client = PirClient::new(elements.len() as i32)?;
    let server1 = PirServer::new(&elements)?;
    let server2 = PirServer::new(&elements)?;
    
    let indices = vec![5];
    let Request { request1, request2 } = client.generate_requests(&indices)?;
    
    assert!(!request1.is_empty());
    assert!(!request2.is_empty());
    
    let response1 = server1.process_request(&request1)?;
    let response2 = server2.process_request(&request2)?;
    
    assert!(!response1.is_empty());
    assert!(!response2.is_empty());
    
    let final_response = client.process_responses(Response { 
        response1, 
        response2 
    })?;
    
    assert_eq!(final_response, "Element5");
    Ok(())
}

#[test]
fn test_error_handling() {
    assert!(matches!(
        PirClient::new(-1),
        Err(PirError::InvalidArgument)
    ));
    
    let empty_elements: Vec<String> = vec![];
    assert!(matches!(
        PirServer::new(&empty_elements),
        Err(PirError::InvalidArgument)
    ));
    
    let client = PirClient::new(4).unwrap();
    let out_of_bounds = vec![10];
    assert!(matches!(
        client.generate_requests(&out_of_bounds),
        Err(PirError::InvalidArgument)
    ));
}