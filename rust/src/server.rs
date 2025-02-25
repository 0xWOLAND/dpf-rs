use cuckoo::{prf, Item, Table};
use libc::{c_char, c_int, c_void};
use rand::{thread_rng, Rng, RngCore};
use std::ffi::{CStr, CString};
use std::ptr;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::{
    constants::{BUCKET_DEPTH, RANDOM_SEED},
    error::{PirError, PirStatus},
};

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

pub struct PirServer {
    handle: *mut c_void,
    elements: Vec<String>,
    capacity: usize,
}


impl PirServer {
    pub fn new(capacity: usize, item_size: usize) -> Result<Self, PirError> {
        if capacity == 0 {
            return Err(PirError::InvalidArgument);
        }

        let mut rng = thread_rng();
        let elements: Vec<String> = (0..capacity).map(|_| {
            let mut data = vec![0u8; item_size];
            rng.fill_bytes(&mut data);
            BASE64.encode(data)
        }).collect();
        
        unsafe {
            let c_strings: Vec<CString> = elements
                .iter()
                .map(|element| {
                    CString::new(element.clone())
                        .map_err(|_| PirError::InvalidArgument)
                })
                .collect::<Result<Vec<_>, _>>()?;

            let c_ptrs: Vec<*const c_char> = c_strings.iter().map(|cs| cs.as_ptr()).collect();
            let mut handle = ptr::null_mut();
            let result: Result<(), PirError> =
                pir_server_create(c_ptrs.as_ptr(), capacity as c_int, &mut handle).into();
            result.map(|_| Self {
                handle,
                elements,
                capacity,
            })
        }
    }

    pub fn write(&mut self, index: usize, element: String) -> Result<(), PirError> {
        self.batch_write(&[(index, element)])
    }
    
    pub fn batch_write(&mut self, updates: &[(usize, String)]) -> Result<(), PirError> {
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
                .map(|element| {
                    CString::new(element.clone())
                        .map_err(|_| PirError::InvalidArgument)
                })
                .collect::<Result<Vec<_>, _>>()?;

            let c_ptrs: Vec<*const c_char> = c_strings.iter().map(|cs| cs.as_ptr()).collect();
            let mut new_handle = ptr::null_mut();
            let result: Result<(), PirError> =
                pir_server_create(c_ptrs.as_ptr(), self.capacity as c_int, &mut new_handle).into();
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
            let c_request = CString::new(request_base64).map_err(|_| PirError::InvalidArgument)?;

            let mut response_ptr = ptr::null_mut();
            let result: Result<(), PirError> =
                pir_server_process_request(self.handle, c_request.as_ptr(), &mut response_ptr)
                    .into();
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

    pub fn get_elements(&self) -> &[String] {
        &self.elements
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for PirServer {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                pir_server_destroy(self.handle);
            }
        }
    }
}

pub struct Server {
    pir: PirServer,
    table: Table,
}

impl Server {
    pub fn new(capacity: usize, item_size: usize) -> Result<Self, PirError> {
        if capacity == 0 {
            return Err(PirError::InvalidArgument);
        }
        let table = Table::new(
            capacity,
            BUCKET_DEPTH,
            item_size,
            Some(vec![0u8; capacity * BUCKET_DEPTH * item_size]),
            RANDOM_SEED,
            TEST_KEY1.to_vec(),
            TEST_KEY2.to_vec(),
        )
        .ok_or(PirError::InvalidArgument)?;
        let pir = PirServer::new(capacity, item_size)?;

        Ok(Self { pir, table })
    }

    pub fn write(&mut self, element: Vec<u8>, seq_no: u64) -> Result<(), PirError> {
        self.batch_write(&[(element, seq_no)])
    }

    pub fn batch_write(&mut self, updates: &[(Vec<u8>, u64)]) -> Result<(), PirError> {
        let mut rng = thread_rng();
        for (element, seq_no) in updates {
            let bucket1 = prf(&self.table.key1, *seq_no).unwrap() % self.table.num_buckets;
            let bucket2 = prf(&self.table.key2, *seq_no).unwrap() % self.table.num_buckets;
            
            let item = Item::new(
                rng.gen::<u64>(),
                element.clone(),
                *seq_no,
                bucket1,
                bucket2
            );
            if self.table.insert(&item).is_err() {
                return Err(PirError::TableFull);
            }
        }

        self.update_pir_data()
    }

    pub fn get(&self, request_base64: &String) -> Result<String, PirError> {
        self.pir.process_request(request_base64)
    }

    fn update_pir_data(&mut self) -> Result<(), PirError> {
        let bucket_size = BUCKET_DEPTH * self.table.item_size;
        
        // Each update contains the entire bucket's data
        let updates: Vec<(usize, String)> = (0..self.table.num_buckets)
            .map(|bucket_idx| {
                let start = bucket_idx * bucket_size;
                let data = &self.table.data[start..start + bucket_size];
                let encoded = BASE64.encode(data);
                (bucket_idx, encoded)
            })
            .collect();

        self.pir.batch_write(&updates)
    }

    pub fn get_elements(&self) -> &[String] {
        self.pir.get_elements()
    }

    pub fn capacity(&self) -> usize {
        self.pir.capacity()
    }

    pub fn key1(&self) -> &[u8] {
        &self.table.key1
    }

    pub fn key2(&self) -> &[u8] {
        &self.table.key2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    const TEST_ITEM_SIZE: usize = 64;
    const TEST_CAPACITY: usize = 4;

    fn create_test_data(size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        thread_rng().fill_bytes(&mut data);
        data
    }

    #[test]
    fn test_pir_server_creation() {
        let capacity = 3;
        let server = PirServer::new(capacity, TEST_ITEM_SIZE);
        assert!(server.is_ok());
        if let Ok(server) = server {
            assert_eq!(server.capacity(), capacity);
            assert_eq!(server.get_elements().len(), capacity);
        }
        let server = PirServer::new(0, TEST_ITEM_SIZE);
        assert!(matches!(server, Err(PirError::InvalidArgument)));
    }

    #[test]
    fn test_server_creation() {
        // Test valid creation
        let server = Server::new(TEST_CAPACITY, TEST_ITEM_SIZE);
        assert!(server.is_ok());
        if let Ok(server) = server {
            assert_eq!(server.capacity(), TEST_CAPACITY);
        }

        // Test creation with zero capacity
        let server = Server::new(0, TEST_ITEM_SIZE);
        assert!(matches!(server, Err(PirError::InvalidArgument)));
    }

    #[test]
    fn test_server_write() {
        let mut server = Server::new(TEST_CAPACITY, TEST_ITEM_SIZE).unwrap();

        // Test writing valid element at valid index
        let new_element = create_test_data(TEST_ITEM_SIZE);
        assert!(server.write(new_element, 1).is_ok());
    }

    #[test]
    fn test_byte_data_preservation() {
        let mut server = Server::new(TEST_CAPACITY, TEST_ITEM_SIZE).unwrap();
        
        // Test with specific byte patterns
        let test_patterns = vec![
            vec![0u8; TEST_ITEM_SIZE],                    // All zeros
            vec![255u8; TEST_ITEM_SIZE],                  // All max bytes
            (0..TEST_ITEM_SIZE as u8).collect(),          // Sequential bytes
            create_test_data(TEST_ITEM_SIZE),             // Random bytes
        ];

        for (i, pattern) in test_patterns.iter().enumerate() {
            assert!(server.write(pattern.clone(), i as u64).is_ok());
        }
    }

    #[test]
    fn test_null_byte_handling() {
        let mut server = Server::new(TEST_CAPACITY, TEST_ITEM_SIZE).unwrap();
        
        // Create test data with explicit null bytes
        let mut data = vec![1u8; TEST_ITEM_SIZE];
        data[0] = 0;  // First byte null
        data[TEST_ITEM_SIZE/2] = 0;  // Middle byte null
        data[TEST_ITEM_SIZE-1] = 0;  // Last byte null
        
        assert!(server.write(data, 1).is_ok());
    }
}