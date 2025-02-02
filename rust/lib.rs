pub mod database;
pub mod server;
pub mod client;

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::sync::Arc;

//     #[derive(Debug)]
//     struct TestError(String);

//     impl std::error::Error for TestError {}

//     impl std::fmt::Display for TestError {
//         fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//             write!(f, "{}", self.0)
//         }
//     }

//     impl From<server::DpfError> for TestError {
//         fn from(err: server::DpfError) -> Self {
//             TestError(err.to_string())
//         }
//     }

//     impl From<std::string::FromUtf8Error> for TestError {
//         fn from(err: std::string::FromUtf8Error) -> Self {
//             TestError(err.to_string())
//         }
//     }

//     fn create_test_database() -> Result<Arc<database::Database>, TestError> {
//         let config = server::DpfPirConfig {
//             database_size: 5,
//         };

//         let mut builder = database::DatabaseBuilder::new()
//             .map_err(|e| TestError(e.to_string()))?;
        
//         // Add test values
//         let values = vec![
//             "value1".as_bytes().to_vec(),
//             "value2".as_bytes().to_vec(),
//             "value3".as_bytes().to_vec(),
//             "value4".as_bytes().to_vec(),
//             "value5".as_bytes().to_vec(),
//         ];

//         for value in &values {
//             builder.insert(value).map_err(|e| TestError(e.to_string()))?;
//         }

//         let db = builder.build().map_err(|e| TestError(e.to_string()))?;
//         Ok(Arc::new(db))
//     }

//     #[test]
//     fn test_pir_end_to_end() -> Result<(), TestError> {
//         // Create database
//         let database = create_test_database()?;

//         // // Create server config
//         // let config = server::DpfPirConfig {
//         //     database_size: 5,
//         // };

//         // // Create leader server
//         // let database_clone = Arc::clone(&database);
//         // let forward_fn = move |request: &[u8], _context: &str| -> Result<Vec<u8>, server::DpfError> {
//         //     Ok(request.to_vec())
//         // };
        
//         // let leader_server = server::Server::new_leader(
//         //     &config,
//         //     Arc::as_ptr(&database_clone) as *mut _,
//         //     forward_fn,
//         // )?;

//         // // Create helper server
//         // let database_clone = Arc::clone(&database);
//         // let decrypt_fn = move |ciphertext: &[u8], _context: &str| -> Result<Vec<u8>, server::DpfError> {
//         //     Ok(ciphertext.to_vec())
//         // };
        
//         // let helper_server = server::Server::new_helper(
//         //     &config,
//         //     Arc::as_ptr(&database_clone) as *mut _,
//         //     decrypt_fn,
//         // )?;

//         // // Create client
//         // let client = client::DpfPirClient::new(&config)?;

//         // // Create request
//         // let indices = vec![2];  // Request the third value
//         // let request = client.create_request(&indices)?;

//         // // Get responses
//         // let leader_response = leader_server.handle_request(&request)?;
//         // let helper_response = helper_server.handle_request(&request)?;

//         // // Combine responses
//         // let mut combined_response = vec![];
//         // combined_response.extend_from_slice(&leader_response);
//         // combined_response.extend_from_slice(&helper_response);

//         // // Handle response
//         // let result = client.handle_response(&combined_response)?;
//         // assert_eq!(String::from_utf8(result[0].clone())?, "value3");

//         Ok(())
//     }
// }

#[test]
fn test() {
    println!("here");
}