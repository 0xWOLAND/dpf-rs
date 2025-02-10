#[cfg(test)]
mod test {
    use dpf_rs::{
        client::{PirClient, Request, Response},
        server::PirServer,
        PirError,
    };

    fn setup_servers(initial_elements: usize) -> (PirClient, PirServer<String>, PirServer<String>) {
        let elements: Vec<String> = (0..initial_elements).map(|i| format!("Element{}", i)).collect();
        let client = PirClient::new(elements.len() as i32).unwrap();
        let server1 = PirServer::new(&elements).unwrap();
        let server2 = PirServer::new(&elements).unwrap();
        (client, server1, server2)
    }

    #[test]
    fn test_single_element_query() -> Result<(), PirError> {
        let elements: Vec<String> = (0..4).map(|i| format!("Element{}", i)).collect();
        
        let client = PirClient::new(elements.len() as i32)?;
        let server1 = PirServer::new(&elements)?;
        let server2 = PirServer::new(&elements)?;
        
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
        let (client, server1, server2) = setup_servers(4);
        
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
    fn test_server_write() -> Result<(), PirError> {
        let (mut client, mut server1, mut server2) = setup_servers(4);

        // Update client size for the new element
        client.update_size(5)?;

        // Add a new element to both servers
        server1.write("NewElement".to_string())?;
        server2.write("NewElement".to_string())?;

        // Query for the newly added element
        let Request { request1, request2 } = client.generate_requests(&[4])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let final_response = client.process_responses(Response { response1, response2 })?;
        
        assert_eq!(final_response, "NewElement");
        assert_eq!(server1.get_elements().len(), 5);
        assert_eq!(server2.get_elements().len(), 5);
        Ok(())
    }

    #[test]
    fn test_server_batch_write() -> Result<(), PirError> {
        let (mut client, mut server1, mut server2) = setup_servers(4);

        // Update client size for the new elements
        client.update_size(6)?;

        // Add multiple new elements to both servers
        let new_elements = vec!["NewElement1".to_string(), "NewElement2".to_string()];
        server1.batch_write(&new_elements)?;
        server2.batch_write(&new_elements)?;

        // Query for both newly added elements
        let indices = vec![4, 5];
        let Request { request1, request2 } = client.generate_requests(&indices)?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let final_response = client.process_responses(Response { response1, response2 })?;
        
        assert_eq!(final_response, "NewElement1, NewElement2");
        assert_eq!(server1.get_elements().len(), 6);
        assert_eq!(server2.get_elements().len(), 6);
        Ok(())
    }

    #[test]
    fn test_write_then_query() -> Result<(), PirError> {
        let (mut client, mut server1, mut server2) = setup_servers(4);

        // First query original element
        let Request { request1, request2 } = client.generate_requests(&[1])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let initial_response = client.process_responses(Response { response1, response2 })?;
        assert_eq!(initial_response, "Element1");

        // Update client size and write new element
        client.update_size(5)?;
        server1.write("NewElement".to_string())?;
        server2.write("NewElement".to_string())?;

        let Request { request1, request2 } = client.generate_requests(&[4])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let final_response = client.process_responses(Response { response1, response2 })?;
        assert_eq!(final_response, "NewElement");

        Ok(())
    }

    #[test]
    fn test_update_size() -> Result<(), PirError> {
        let (mut client, mut server1, mut server2) = setup_servers(4);
        
        // Test that current size works
        let Request { request1, request2 } = client.generate_requests(&[1])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let response = client.process_responses(Response { response1, response2 })?;
        assert_eq!(response, "Element1");

        // Update to larger size
        client.update_size(8)?;
        
        // Write new elements
        server1.write("NewElement1".to_string())?;
        server2.write("NewElement1".to_string())?;
        
        // Test querying new index works
        let Request { request1, request2 } = client.generate_requests(&[4])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let response = client.process_responses(Response { response1, response2 })?;
        assert_eq!(response, "NewElement1");

        // Test invalid size update
        assert!(matches!(
            client.update_size(-1),
            Err(PirError::InvalidArgument)
        ));
        
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
}