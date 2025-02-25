use cuckoo::{prf, Item, Table};
use libc::{c_char, c_int, c_void};
use rand::{thread_rng, Rng, RngCore};
use std::ffi::{CStr, CString};
use std::ptr;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::{
    constants::{BUCKET_DEPTH, RANDOM_SEED, PADDING_SIZE, NONCE_SIZE},
    error::{PirError, PirStatus},
};

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

        let elements: Vec<String> = (0..capacity).map(|_| {
            BASE64.encode(vec![0u8; item_size])
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
            item_size + PADDING_SIZE + NONCE_SIZE,
            Some(vec![0u8; capacity * BUCKET_DEPTH * (item_size + PADDING_SIZE + NONCE_SIZE)]),
            RANDOM_SEED,
        )
        .ok_or(PirError::InvalidArgument)?;
        let pir = PirServer::new(capacity, item_size + PADDING_SIZE + NONCE_SIZE)?;

        Ok(Self { pir, table })
    }

    pub fn write(&mut self, item: Item) -> Result<(), PirError> {
        self.batch_write(&[item])
    }

    pub fn batch_write(&mut self, updates: &[Item]) -> Result<(), PirError> {
        for item in updates {
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
}