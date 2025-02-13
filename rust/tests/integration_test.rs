#[cfg(test)]
mod test {
    use dpf_rs::{
        client::{PirClient, Request, Response},
        server::PirServer,
        PirError,
    };

    fn setup_servers(capacity: usize) -> (PirClient, PirServer<String>, PirServer<String>) {
        let default_value = String::from("");
        let client = PirClient::new(capacity as i32).unwrap();
        let mut server1 = PirServer::new(capacity, &default_value).unwrap();
        let mut server2 = PirServer::new(capacity, &default_value).unwrap();

        // Initialize with test data
        let elements: Vec<(usize, String)> = (0..capacity)
            .map(|i| (i, format!("Element{}", i)))
            .collect();
        server1.batch_write(&elements).unwrap();
        server2.batch_write(&elements).unwrap();

        (client, server1, server2)
    }

    #[test]
    fn test_single_element_query() -> Result<(), PirError> {
        let (client, server1, server2) = setup_servers(4);
        
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
        let (client, mut server1, mut server2) = setup_servers(5);

        // Write a new element at index 4
        server1.write(4, "NewElement".to_string())?;
        server2.write(4, "NewElement".to_string())?;

        // Query for the newly written element
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
        let (client, mut server1, mut server2) = setup_servers(6);

        // Write multiple elements at specific indices
        let updates = vec![
            (4, "NewElement1".to_string()),
            (5, "NewElement2".to_string())
        ];
        server1.batch_write(&updates)?;
        server2.batch_write(&updates)?;

        // Query for both newly written elements
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
        let (client, mut server1, mut server2) = setup_servers(5);

        // First query original element
        let Request { request1, request2 } = client.generate_requests(&[1])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let initial_response = client.process_responses(Response { response1, response2 })?;
        assert_eq!(initial_response, "Element1");

        // Write new element at index 4
        server1.write(4, "NewElement".to_string())?;
        server2.write(4, "NewElement".to_string())?;

        let Request { request1, request2 } = client.generate_requests(&[4])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let final_response = client.process_responses(Response { response1, response2 })?;
        assert_eq!(final_response, "NewElement");

        Ok(())
    }

    #[test]
    fn test_error_handling() {
        // Test invalid capacity
        let default_value = String::from("");
        assert!(matches!(
            PirServer::new(0, &default_value),
            Err(PirError::InvalidArgument)
        ));
        
        let (_, mut server1, _) = setup_servers(4);
        
        // Test out of bounds write
        assert!(matches!(
            server1.write(10, "OutOfBounds".to_string()),
            Err(PirError::IndexOutOfBounds)
        ));
        
        // Test out of bounds batch write
        assert!(matches!(
            server1.batch_write(&[(5, "OutOfBounds".to_string())]),
            Err(PirError::IndexOutOfBounds)
        ));

        // Test client error cases
        assert!(matches!(
            PirClient::new(-1),
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