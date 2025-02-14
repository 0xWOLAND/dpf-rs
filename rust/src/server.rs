use base64::engine::{general_purpose::STANDARD as BASE64, Engine as _};
use cuckoo::{prf, Item, Table};
use libc::{c_char, c_int, c_void};
use rand::{thread_rng, Rng};
use std::ffi::{CStr, CString};
use std::ptr;

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
    T: Into<Vec<u8>> + Clone + Default,
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
                    let bytes: Vec<u8> = element.clone().into();
                    Ok(CString::new(bytes).expect("Cstring failed"))
                    // CString::new(bytes).map_err(|_| PirError::InvalidArgument)
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
                    CString::new(bytes).map_err(|_| PirError::InvalidArgument)
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

    // Rest remains the same
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
    pir: PirServer<Vec<u8>>,
    table: Table,
}

impl Server {
    pub fn new(capacity: usize, item_size: usize) -> Result<Self, PirError> {
        if capacity == 0 {
            return Err(PirError::InvalidArgument);
        }

        let mut rng = thread_rng();
        let chunk_size = BUCKET_DEPTH * item_size;
        let rand_chunk = vec![1u8; chunk_size];

        // Initialize empty table
        let table = Table::new(
            capacity,
            BUCKET_DEPTH,
            item_size,
            Some(
                vec![rand_chunk.clone(); capacity]
                    .into_iter()
                    .flatten()
                    .collect(),
            ),
            RANDOM_SEED,
            TEST_KEY1.to_vec(),
            TEST_KEY2.to_vec(),
        )
        .ok_or(PirError::InvalidArgument)?;

        // Initialize PIR server with empty chunks
        let pir = PirServer::new(capacity, &rand_chunk).expect("PIR Server failed here");

        Ok(Self { pir, table })
    }

    pub fn write(&mut self, index: usize, element: Vec<u8>) -> Result<(), PirError> {
        self.batch_write(&[(index, element)])
    }

    pub fn batch_write(&mut self, updates: &[(usize, Vec<u8>)]) -> Result<(), PirError> {
        // Validate updates
        for (index, element) in updates {
            if *index >= self.table.num_buckets {
                return Err(PirError::IndexOutOfBounds);
            }
            if element.len() != self.table.item_size {
                return Err(PirError::InvalidArgument);
            }
        }

        // Insert items into table
        let mut rng = thread_rng();
        for (index, element) in updates {
            let seq_no = rng.gen::<u64>();
            let bucket1 = prf(&self.table.key1, seq_no).unwrap() % self.table.num_buckets;
            let bucket2 = prf(&self.table.key2, seq_no).unwrap() % self.table.num_buckets;
            let item = Item::new(rng.gen::<u64>(), element.clone(), seq_no, bucket1, bucket2);
            if self
                .table
                .insert(&item)
                .is_err()
            {
                return Err(PirError::TableFull);
            }
        }

        self.update_pir_data()
    }

    pub fn get(&self, request_base64: &str) -> Result<String, PirError> {
        self.pir.process_request(request_base64)
    }

    fn update_pir_data(&mut self) -> Result<(), PirError> {
        let chunk_size = self.table.data.len() / self.table.num_buckets;

        let updates: Vec<(usize, Vec<u8>)> = self
            .table
            .data
            .chunks_exact(chunk_size)
            .enumerate()
            .map(|(index, chunk)| (index, chunk.to_vec()))
            .collect();

        self.pir.batch_write(&updates)
    }

    pub fn capacity(&self) -> usize {
        self.table.num_buckets
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
        let server = PirServer::new(capacity, &default_value);
        assert!(server.is_ok());
        if let Ok(server) = server {
            assert_eq!(server.capacity(), capacity);
            assert_eq!(server.get_elements().len(), capacity);
        }
        let server = PirServer::new(0, &default_value);
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
        assert!(server.write(0, new_element).is_ok());

        // Test writing element with wrong size
        let invalid_element = create_test_data(TEST_ITEM_SIZE - 1);
        assert!(matches!(
            server.write(0, invalid_element),
            Err(PirError::InvalidArgument)
        ));

        // Test writing at invalid index
        let element = create_test_data(TEST_ITEM_SIZE);
        assert!(matches!(
            server.write(TEST_CAPACITY, element),
            Err(PirError::IndexOutOfBounds)
        ));
    }

    #[test]
    fn test_server_batch_write() {
        let mut server = Server::new(TEST_CAPACITY, TEST_ITEM_SIZE).unwrap();

        // Test valid batch write
        let updates = vec![
            (0, create_test_data(TEST_ITEM_SIZE)),
            (2, create_test_data(TEST_ITEM_SIZE)),
        ];
        assert!(server.batch_write(&updates).is_ok());

        // Test batch write with invalid index
        let invalid_updates = vec![
            (0, create_test_data(TEST_ITEM_SIZE)),
            (TEST_CAPACITY, create_test_data(TEST_ITEM_SIZE)), // Invalid index
        ];
        assert!(matches!(
            server.batch_write(&invalid_updates),
            Err(PirError::IndexOutOfBounds)
        ));

        // Test batch write with invalid size
        let invalid_updates = vec![
            (0, create_test_data(TEST_ITEM_SIZE)),
            (1, create_test_data(TEST_ITEM_SIZE - 1)), // Invalid size
        ];
        assert!(matches!(
            server.batch_write(&invalid_updates),
            Err(PirError::InvalidArgument)
        ));
    }
}
