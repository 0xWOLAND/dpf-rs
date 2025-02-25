use libc::{c_char, c_int, c_void};
use std::{ffi::{CStr, CString}, ptr};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use cuckoo::{prf as cuckoo_prf, Item};
use rand::{thread_rng, Rng};

use crate::{error::{PirError, PirStatus, CryptoError}, utils::{Key, kdf, encrypt, decrypt}, constants::PADDING_SIZE};

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub request1: String,
    pub request2: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub response1: String,
    pub response2: String,
}

#[link(name = "dpf_client")]
extern "C" {
    fn pir_client_create(database_size: c_int, client_handle: *mut *mut c_void) -> PirStatus;

    fn pir_client_generate_requests(
        client_handle: *mut c_void,
        indices: *const c_int,
        num_indices: c_int,
        requests_json: *mut *mut c_char,
    ) -> PirStatus;

    fn pir_client_process_responses(
        responses_json: *const c_char,
        merged_result: *mut *mut c_char,
    ) -> PirStatus;

    fn pir_client_free_string(str: *mut c_char);
    fn pir_client_destroy(client_handle: *mut c_void);
}

pub struct Client {
    id: String,
    handle: *mut c_void,
    database_size: i32,
    keys: HashMap<String, (Key, Key, Key)>,
}

impl Client {
    pub fn new(id: String, database_size: i32) -> Result<Self, PirError> {
        if database_size <= 0 {
            return Err(PirError::InvalidArgument);
        }

        unsafe {
            let mut handle = ptr::null_mut();
            let result: Result<(), PirError> = pir_client_create(database_size, &mut handle).into();
            result.map(|_| Self { id, handle, database_size, keys: HashMap::new() })
        }
    }

    pub fn update_size(&mut self, new_size: i32) -> Result<(), PirError> {
        if new_size <= 0 {
            return Err(PirError::InvalidArgument);
        }

        let keys = self.keys.clone();
        let mut new_client = Client::new(self.id.clone(), new_size)?;
        new_client.keys = keys;
        
        unsafe {
            if !self.handle.is_null() {
                pir_client_destroy(self.handle);
            }
            self.handle = new_client.handle;
            std::mem::forget(new_client);
        }
        
        Ok(())
    }

    pub fn add_key(&mut self, to: String, key: Key) -> Result<(), PirError> {
        let key1 = kdf(&key, "key1").unwrap();
        let key2 = kdf(&key, "key2").unwrap();
        let k_enc = kdf(&key, "k_enc").unwrap();
        
        self.keys.insert(to, (key1, key2, k_enc));

        Ok(())
    }

    pub fn encrypt(&self, to: String, element: Vec<u8>) -> Result<Vec<u8>, PirError> {
        let k_enc = self.keys.get(&to).unwrap().2.clone();
        let encrypted_element = encrypt(&k_enc, &element, PADDING_SIZE).unwrap();
        Ok(encrypted_element)
    }

    pub fn decrypt(&self, to: String, response: Vec<Vec<u8>>) -> Result<Vec<u8>, PirError> {
        let k_enc = self.keys.get(&to).unwrap().2.clone();
        
        for bucket in response {
            for chunk in bucket.chunks(bucket.len() / 4) {
                println!("chunk: {:?}", chunk);
                if let Ok(decrypted_chunk) = decrypt(&k_enc, chunk) {
                    return Ok(decrypted_chunk);
                }
            }
        }
        
        println!("Decryption failed");
        Err(PirError::Crypto(CryptoError::DecryptionFailed))
    }


    pub fn generate_requests(&self, to: String, element: Vec<u8>, seq_no: u64) -> Result<(Item, Request), PirError> {
        let mut rng = thread_rng();
        let id = rng.gen::<u64>();
        let key1 = self.keys.get(&to).unwrap().0.clone();
        let key2 = self.keys.get(&to).unwrap().1.clone();
        
        let bucket1 = cuckoo_prf(key1.as_slice(), seq_no).unwrap() % self.database_size as usize;
        let bucket2 = cuckoo_prf(key2.as_slice(), seq_no).unwrap() % self.database_size as usize;
        let item = Item::new(id, element, bucket1, bucket2);
        
        self._generate_requests(&[bucket1 as i32, bucket2 as i32]).map(|request| (item, request))
    }

    pub fn _generate_requests(&self, indices: &[i32]) -> Result<Request, PirError> {
        unsafe {
            let mut requests_json = ptr::null_mut();
            let result: Result<(), PirError> = pir_client_generate_requests(
                self.handle,
                indices.as_ptr(),
                indices.len() as c_int,
                &mut requests_json,
            ).into();

            result.and_then(|_| {
                if requests_json.is_null() {
                    return Err(PirError::FfiError);
                }
                let result = CStr::from_ptr(requests_json)
                    .to_str()
                    .map(String::from)
                    .map_err(|_| PirError::Utf8Error)?;
                pir_client_free_string(requests_json);
                serde_json::from_str(&result).map_err(|_| PirError::Processing)
            })
        }
    }

    pub fn _process_responses(&self, response: Response) -> Result<String, PirError> {
        unsafe {
            let responses_json = serde_json::to_string(&response)
                .map_err(|_| PirError::Processing)?;
            let c_responses = CString::new(responses_json)
                .map_err(|_| PirError::InvalidArgument)?;
            let mut merged_result = ptr::null_mut();
            let result: Result<(), PirError> = pir_client_process_responses(
                c_responses.as_ptr(), 
                &mut merged_result
            ).into();
            result.and_then(|_| {
                if merged_result.is_null() {
                    return Err(PirError::FfiError);
                }
                let result = CStr::from_ptr(merged_result)
                    .to_str()
                    .map(String::from)
                    .map_err(|_| PirError::Utf8Error)?;
                pir_client_free_string(merged_result);
                Ok(result)
            })
        }
    }

    pub fn process_responses(&self, response: Response) -> Result<Vec<Vec<u8>>, PirError> {
        self._process_responses(response)
            .map(|result| {
                result.split(',')
                    .map(|part| BASE64.decode(part.trim()).unwrap())
                    .collect()
            })
            .map_err(|_| PirError::Processing)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                pir_client_destroy(self.handle);
            }
        }
    }
}
