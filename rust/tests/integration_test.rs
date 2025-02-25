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

    #[test]
    fn test_server_write_and_read() -> Result<(), PirError> {
        // Initialize with some test data
        let item_size = 64;
        let table_size = 10;
        
        let key1 = Key::new_random();
        let key2 = Key::new_random();

        // Create two servers with initial elements
        let mut server1 = Server::new(table_size, item_size)?;
        let mut server2 = Server::new(table_size, item_size)?;

        // Create client
        let client = Client::new(table_size as i32, key1.to_vec(), key2.to_vec())?;

        // Create new element and write it
        let new_element = {
            let mut data = vec![0u8; item_size];
            for i in 0..item_size {
                data[i] = (i % 256) as u8;
            }
            data
        };


        // Convert to string and write to servers
        let new_element_str = new_element.clone();
        let (item, Request { request1, request2 }) = client.generate_requests(new_element.clone(), 0, 0)?;

        server1.write(item.clone())?;
        server2.write(item.clone())?;

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