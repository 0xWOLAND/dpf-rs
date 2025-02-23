use cuckoo::{prf, Item, Table};
use libc::{c_char, c_int, c_void};
use rand::{thread_rng, Rng};
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

pub struct PirServer<T> {
    handle: *mut c_void,
    elements: Vec<T>,
    capacity: usize,
}


impl<T> PirServer<T>
where
    T: Into<Vec<u8>> + Clone + Default 
{
    pub fn new(capacity: usize, default_value: &T) -> Result<Self, PirError> {
        if capacity == 0 {
            return Err(PirError::InvalidArgument);
        }
        let elements = vec![default_value.clone(); capacity];
        unsafe {
            let c_strings: Vec<CString> = elements
                .iter()
                .map(|element| {
                    CString::new(BASE64.encode(element.clone().into()))
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
                .map(|element| {
                    let bytes: Vec<u8> = element.clone().into();
                    // Convert bytes to string using utf8_lossy
                    let string_value = String::from_utf8_lossy(&bytes).to_string();
                    // Escape null bytes before creating CString
                    CString::new(string_value.replace('\0', "�"))
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
    pub fn new(capacity: usize, item_size: usize) -> Result<Self, PirError> {
        if capacity == 0 {
            return Err(PirError::InvalidArgument);
        }

        // Create empty chunk as zeroed bytes for the table
        let chunk_bytes = vec![0u8; BUCKET_DEPTH * item_size];
        
        // Initialize empty table with raw bytes
        let table = Table::new(
            capacity,
            BUCKET_DEPTH,
            item_size,
            Some(vec![chunk_bytes.clone(); capacity].into_iter().flatten().collect()),
            RANDOM_SEED,
            TEST_KEY1.to_vec(),
            TEST_KEY2.to_vec(),
        )
        .ok_or(PirError::InvalidArgument)?;

        // Convert bytes to string for PIR server, replacing nulls with placeholder
        let chunk_str = String::from_utf8_lossy(&chunk_bytes).to_string();
        let pir = PirServer::new(capacity, &chunk_str)?;

        Ok(Self { pir, table })
    }

    pub fn write(&mut self, index: usize, element: Vec<u8>, seq_no: u64) -> Result<(), PirError> {
        self.batch_write(&[(index, element, seq_no)])
    }

    pub fn batch_write(&mut self, updates: &[(usize, Vec<u8>, u64)]) -> Result<(), PirError> {
        // Validate updates
        for (index, element, seq_no) in updates {
            if *index >= self.table.num_buckets {
                return Err(PirError::IndexOutOfBounds);
            }
            if element.len() != self.table.item_size {
                return Err(PirError::InvalidArgument);
            }
        }

        let mut rng = thread_rng();
        for (index, element, seq_no) in updates {
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
    

    pub fn capacity(&self) -> usize {
        self.pir.capacity()
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
        let default_value = vec![1u8; TEST_ITEM_SIZE];
        let capacity = 3;
        let default_str = String::from_utf8_lossy(&default_value).to_string();
        let server = PirServer::new(capacity, &default_str);
        assert!(server.is_ok());
        if let Ok(server) = server {
            assert_eq!(server.capacity(), capacity);
            assert_eq!(server.get_elements().len(), capacity);
        }
        let server = PirServer::new(0, &default_str);
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
        assert!(server.write(0, new_element, 1).is_ok());

        // Test writing element with wrong size
        let invalid_element = create_test_data(TEST_ITEM_SIZE - 1);
        assert!(matches!(
            server.write(0, invalid_element, 2),
            Err(PirError::InvalidArgument)
        ));

        // Test writing at invalid index
        let element = create_test_data(TEST_ITEM_SIZE);
        assert!(matches!(
            server.write(TEST_CAPACITY, element, 3),
            Err(PirError::IndexOutOfBounds)
        ));
    }

    #[test]
    fn test_server_batch_write() {
        let mut server = Server::new(TEST_CAPACITY, TEST_ITEM_SIZE).unwrap();

        // Test valid batch write
        let updates = vec![
            (0, create_test_data(TEST_ITEM_SIZE), 1),
            (2, create_test_data(TEST_ITEM_SIZE), 2),
        ];
        assert!(server.batch_write(&updates).is_ok());

        // Test batch write with invalid index
        let invalid_updates = vec![
            (0, create_test_data(TEST_ITEM_SIZE), 3),
            (TEST_CAPACITY, create_test_data(TEST_ITEM_SIZE), 4), // Invalid index
        ];
        assert!(matches!(
            server.batch_write(&invalid_updates),
            Err(PirError::IndexOutOfBounds)
        ));

        // Test batch write with invalid size
        let invalid_updates = vec![
            (0, create_test_data(TEST_ITEM_SIZE), 5),
            (1, create_test_data(TEST_ITEM_SIZE - 1), 6), // Invalid size
        ];
        assert!(matches!(
            server.batch_write(&invalid_updates),
            Err(PirError::InvalidArgument)
        ));
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
            assert!(server.write(i, pattern.clone(), i as u64).is_ok());
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
        
        assert!(server.write(0, data, 1).is_ok());
    }
}