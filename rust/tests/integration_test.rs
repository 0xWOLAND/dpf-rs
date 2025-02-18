#[cfg(test)]
mod test {
    use rand::{Rng, thread_rng, RngCore};
    use dpf_rs::{
        client::{Client, Request, Response},
        server::{Server, PirServer},
        PirError,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    fn setup_servers(capacity: usize) -> (Client, PirServer<String>, PirServer<String>) {
        let default_value = String::from("");
        let client = Client::new(capacity as i32).unwrap();
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
        
        let final_response = client._process_responses(Response { 
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
        
        let final_response = client._process_responses(Response { 
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
        let final_response = client._process_responses(Response { response1, response2 })?;
        
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
        let final_response = client._process_responses(Response { response1, response2 })?;
        
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
        let initial_response = client._process_responses(Response { response1, response2 })?;
        assert_eq!(initial_response, "Element1");

        // Write new element at index 4
        server1.write(4, "NewElement".to_string())?;
        server2.write(4, "NewElement".to_string())?;

        let Request { request1, request2 } = client.generate_requests(&[4])?;
        let response1 = server1.process_request(&request1)?;
        let response2 = server2.process_request(&request2)?;
        let final_response = client._process_responses(Response { response1, response2 })?;
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
            Client::new(-1),
            Err(PirError::InvalidArgument)
        ));
        
        let client = Client::new(4).unwrap();
        let out_of_bounds = vec![10];
        assert!(matches!(
            client.generate_requests(&out_of_bounds),
            Err(PirError::InvalidArgument)
        ));
    }
    
    #[test]
    fn test_server_write_and_read() -> Result<(), PirError> {
        // Initialize with some test data
        let item_size = 64;
        let initial_elements: Vec<Vec<u8>> = (0..4)
            .map(|_| {
                let mut data = vec![0u8; item_size];
                thread_rng().fill_bytes(&mut data);
                data
            })
            .collect();

        // Store the initial element at index 0 for comparison
        let initial_element_0 = initial_elements[0].clone();

        // Convert initial elements to strings for the server
        let initial_strings: Vec<String> = initial_elements
            .iter()
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .collect();

        // Create two servers with initial elements
        let mut server1 = Server::new(4, item_size)?;
        let mut server2 = Server::new(4, item_size)?;
        
        // Create client
        let client = Client::new(initial_elements.len() as i32)?;

        // Create new element and write it
        let new_element = {
            let mut data = vec![0u8; item_size];
            for i in 0..item_size {
                // data[i] = (i % 256) as u8;
                data[i] = b'a';
            }
            data
        };


        // Convert to string and write to servers
        let new_element_str = new_element.clone();
        server1.write(0, new_element_str.clone())?;
        server2.write(0, new_element_str.clone())?;

        // Create PIR request for index 0
        let Request { request1, request2 } = client.generate_requests(&[0])?;
        
        // Get responses from servers
        let response1 = server1.get(&request1)?;
        let response2 = server2.get(&request2)?;

        // Process responses
        let final_response = client.process_responses(Response {
            response1,
            response2,
        })?;

        // Convert final response string back to bytes for comparison
        println!("final response: {:?}", final_response);
        println!("initial bytes: {:?}", new_element);

        assert!(1 < 0);

        Ok(())
    }
}