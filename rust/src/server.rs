use libc::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::ptr;
use cuckoo::{Table, Item, prf};
use base64::engine::{Engine as _, general_purpose::STANDARD as BASE64};
use rand::{Rng, thread_rng};

use crate::error::{PirError, PirStatus};

// Constants that were missing
const BUCKET_DEPTH: usize = 4;  // Adjust value as needed
const RANDOM_SEED: u64 = 12345; // Adjust value as needed
const TEST_KEY1: [u8; 32] = [1u8; 32]; // Example key, adjust as needed
const TEST_KEY2: [u8; 32] = [2u8; 32]; // Example key, adjust as needed

#[link(name = "dpf_server")]
extern "C" {
    fn pir_server_create(
        elements: *const *const c_char,
        num_elements: c_int,
        server_handle: *mut *mut c_void,
    ) -> PirStatus;
    fn pir_server_process_request(
        server_handle: *mut c_void,
        request_base64: *const c_char,
        response_base64: *mut *mut c_char,
    ) -> PirStatus;
    fn pir_server_free_string(str: *mut c_char);
    fn pir_server_destroy(server_handle: *mut c_void);
}

pub struct PirServer<T> {
    handle: *mut c_void,
    elements: Vec<T>,
    capacity: usize,
}

impl<T> PirServer<T>
where
    T: AsRef<str> + Clone + Default
{
    pub fn new(capacity: usize, default_value: &T) -> Result<Self, PirError> {
        if capacity == 0 {
            return Err(PirError::InvalidArgument);
        }

        let elements = vec![default_value.clone(); capacity];

        unsafe {
            let c_strings: Vec<CString> = elements
                .iter()
                .map(|s| CString::new(s.as_ref()).map_err(|_| PirError::InvalidArgument))
                .collect::<Result<Vec<_>, _>>()?;
            
            let c_ptrs: Vec<*const c_char> = c_strings
                .iter()
                .map(|cs| cs.as_ptr())
                .collect();

            let mut handle = ptr::null_mut();
            let result: Result<(), PirError> = pir_server_create(
                c_ptrs.as_ptr(),
                capacity as c_int,
                &mut handle
            ).into();

            result.map(|_| Self {
                handle,
                elements,
                capacity,
            })
        }
    }

    pub fn write(&mut self, index: usize, element: T) -> Result<(), PirError> {
        self.batch_write(&[(index, element)])
    }

    pub fn batch_write(&mut self, updates: &[(usize, T)]) -> Result<(), PirError> {
        for (index, _) in updates {
            if *index >= self.capacity {
                return Err(PirError::IndexOutOfBounds);
            }
        }

        let mut updated_elements = self.elements.clone();
        for (index, element) in updates {
            updated_elements[*index] = element.clone();
        }

        unsafe {
            let c_strings: Vec<CString> = updated_elements
                .iter()
                .map(|s| CString::new(s.as_ref()).map_err(|_| PirError::InvalidArgument))
                .collect::<Result<Vec<_>, _>>()?;
            
            let c_ptrs: Vec<*const c_char> = c_strings
                .iter()
                .map(|cs| cs.as_ptr())
                .collect();

            let mut new_handle = ptr::null_mut();
            let result: Result<(), PirError> = pir_server_create(
                c_ptrs.as_ptr(),
                self.capacity as c_int,
                &mut new_handle
            ).into();

            result?;

            if !self.handle.is_null() {
                pir_server_destroy(self.handle);
            }

            self.handle = new_handle;
            self.elements = updated_elements;
        }

        Ok(())
    }

    pub fn process_request(&self, request_base64: &str) -> Result<String, PirError> {
        unsafe {
            let c_request = CString::new(request_base64)
                .map_err(|_| PirError::InvalidArgument)?;
            
            let mut response_ptr = ptr::null_mut();
            let result: Result<(), PirError> = pir_server_process_request(
                self.handle,
                c_request.as_ptr(),
                &mut response_ptr
            ).into();

            result.and_then(|_| {
                if response_ptr.is_null() {
                    return Err(PirError::FfiError);
                }
                let result = CStr::from_ptr(response_ptr)
                    .to_str()
                    .map(String::from)
                    .map_err(|_| PirError::Utf8Error)?;
                pir_server_free_string(response_ptr);
                Ok(result)
            })
        }
    }

    pub fn get_elements(&self) -> &[T] {
        &self.elements
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Drop for PirServer<T> {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                pir_server_destroy(self.handle);
            }
        }
    }
}


pub struct Server {
    pir: PirServer<String>,
    table: Table,
}



impl Server {
    pub fn new(elements: &[Vec<u8>], item_size: usize) -> Result<Self, PirError> {
        if elements.is_empty() {
            return Err(PirError::InvalidArgument);
        }

        // Validate all elements have correct size
        if elements.iter().any(|elem| elem.len() != item_size) {
            return Err(PirError::InvalidArgument);
        }

        let num_buckets = elements.len();
        let mut table = Table::new(
            num_buckets,
            BUCKET_DEPTH,
            item_size,
            None,
            RANDOM_SEED,
            TEST_KEY1.to_vec(),
            TEST_KEY2.to_vec(),
        ).ok_or(PirError::InvalidArgument)?;

        // Insert initial elements into table
        let mut rng = thread_rng();
        for element in elements {
            let seq_no = rng.gen::<u64>();
            let bucket1 = prf(&table.key1, seq_no).unwrap() % num_buckets;
            let bucket2 = prf(&table.key2, seq_no).unwrap() % num_buckets;
            
            let item = Item::new(
                rng.gen::<u64>(),
                element.clone(),
                seq_no,
                bucket1,
                bucket2
            );
            
            if table.insert(&item).map_err(|_| PirError::TableFull)?.is_some() {
                return Err(PirError::TableFull);
            }
        }

        // Initialize PIR server with encoded table data
        let chunk_size = table.data.len() / num_buckets;
        let encoded_chunks: Vec<String> = table.data
            .chunks_exact(chunk_size)
            .map(|chunk| BASE64.encode(chunk))
            .collect();

        let pir = PirServer::new(num_buckets, &encoded_chunks[0])?;
        
        // Update PIR server with all chunks
        let updates: Vec<(usize, String)> = encoded_chunks
            .into_iter()
            .enumerate()
            .collect();
            
        let mut server = Self { pir, table };
        server.pir.batch_write(&updates)?;

        Ok(server)
    }

    pub fn write(&mut self, element: Vec<u8>) -> Result<(), PirError> {
        if element.len() != self.table.item_size {
            return Err(PirError::InvalidArgument);
        }
    
        let mut rng = thread_rng();
        let seq_no = rng.gen::<u64>();
        let bucket1 = prf(&self.table.key1, seq_no).unwrap() % self.table.num_buckets;
        let bucket2 = prf(&self.table.key2, seq_no).unwrap() % self.table.num_buckets;
    
        let item = Item::new(
            rng.gen::<u64>(),
            element,
            seq_no,
            bucket1,
            bucket2
        );
    
        match self.table.insert(&item).map_err(|_| PirError::TableFull)? {
            None => {
                self.update_pir_data()?;
                Ok(())
            }
            Some(_) => Err(PirError::TableFull)
        }
    }

    pub fn get(&self, request_base64: &str) -> Result<String, PirError> {
        self.pir.process_request(request_base64)
    }

    fn update_pir_data(&mut self) -> Result<(), PirError> {
        let chunk_size = self.table.data.len() / self.table.num_buckets;
        
        let updates: Vec<(usize, String)> = self.table.data
            .chunks_exact(chunk_size)
            .enumerate()
            .map(|(index, chunk)| (index, BASE64.encode(chunk)))
            .collect();

        self.pir.batch_write(&updates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    const TEST_ITEM_SIZE: usize = 64;

    fn create_test_data(size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        thread_rng().fill_bytes(&mut data);
        data
    }

    #[test]
    fn test_pir_server_creation() {
        let default_value = String::from("");
        let capacity = 3;
        let server = PirServer::new(capacity, &default_value);
        assert!(server.is_ok());

        if let Ok(server) = server {
            assert_eq!(server.capacity(), capacity);
            assert_eq!(server.get_elements().len(), capacity);
        }

        // Test error case with zero capacity
        let server = PirServer::new(0, &default_value);
        assert!(matches!(server, Err(PirError::InvalidArgument)));
    }

    #[test]
    fn test_server_creation() {
        // Create test elements
        let elements: Vec<Vec<u8>> = (0..4)
            .map(|_| create_test_data(TEST_ITEM_SIZE))
            .collect();

        // Test successful creation
        let server = Server::new(&elements, TEST_ITEM_SIZE);
        assert!(server.is_ok());

        // Test creation with empty elements
        let empty_elements: Vec<Vec<u8>> = vec![];
        let server = Server::new(&empty_elements, TEST_ITEM_SIZE);
        assert!(matches!(server, Err(PirError::InvalidArgument)));

        // Test creation with invalid item size
        let invalid_elements: Vec<Vec<u8>> = vec![
            create_test_data(TEST_ITEM_SIZE),
            create_test_data(TEST_ITEM_SIZE - 1), // Wrong size
        ];
        let server = Server::new(&invalid_elements, TEST_ITEM_SIZE);
        assert!(matches!(server, Err(PirError::InvalidArgument)));
    }

    #[test]
    fn test_server_write() {
        // Create initial elements
        let initial_elements: Vec<Vec<u8>> = (0..4)
            .map(|_| create_test_data(TEST_ITEM_SIZE))
            .collect();

        let mut server = Server::new(&initial_elements, TEST_ITEM_SIZE).unwrap();

        // Test writing valid element
        let new_element = create_test_data(TEST_ITEM_SIZE);
        assert!(server.write(new_element).is_ok());

        // Test writing element with wrong size
        let invalid_element = create_test_data(TEST_ITEM_SIZE - 1);
        assert!(matches!(
            server.write(invalid_element),
            Err(PirError::InvalidArgument)
        ));
    }
}