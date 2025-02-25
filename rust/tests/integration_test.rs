#[cfg(test)]
mod test {
    use rand::{Rng, thread_rng, RngCore};
    use dpf_rs::{
        client::{Client, Request, Response},
        server::{Server, PirServer},
        utils::Key,
        PirError,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use cuckoo::prf;

    const TEST_ITEM_SIZE: usize = 64;
    const ITEM_SIZE: usize = 64;
    const TABLE_SIZE: usize = 10;

    #[test]
    fn test_server_write_and_read() -> Result<(), PirError> {
        let key = Key::new_random();

        let mut client1 = Client::new("client1".to_string(), TABLE_SIZE as i32)?;
        let mut client2 = Client::new("client2".to_string(), TABLE_SIZE as i32)?;

        // Create two servers with initial elements
        let mut server1 = Server::new(TABLE_SIZE, ITEM_SIZE)?;
        let mut server2 = Server::new(TABLE_SIZE, ITEM_SIZE)?;

        client1.add_key("client2".to_string(), key.clone())?;
        client2.add_key("client1".to_string(), key.clone())?;

        // Create new element and write it
        let new_element = {
            let mut data = vec![0u8; ITEM_SIZE];
            for i in 0..ITEM_SIZE {
                data[i] = (i % 256) as u8;
            }
            data
        };


        // Convert to string and write to servers
        let encrypted_element = client1.encrypt("client2".to_string(), new_element.clone())?;
        let (item, Request { request1, request2 }) = client1.generate_requests("client2".to_string(), encrypted_element.clone(), 0)?;
        let (item, Request { request1, request2 }) = client2.generate_requests("client1".to_string(), encrypted_element, 0)?;

        server1.write(item.clone())?;
        server2.write(item.clone())?;

        let response1 = server1.get(&request1)?;
        let response2 = server2.get(&request2)?;
        
        // Process responses
        let client1_response = client1.process_responses(Response {
            response1: response1.clone(),
            response2: response2.clone(),
        })?;

        let client2_response = client2.process_responses(Response {
            response1,
            response2,
        })?;

        let decrypted_element1 = client1.decrypt("client2".to_string(), client1_response)?;
        let decrypted_element2 = client2.decrypt("client1".to_string(), client2_response)?;
        
        assert_eq!(decrypted_element1, new_element);
        assert_eq!(decrypted_element2, new_element);

        Ok(())
    }
}