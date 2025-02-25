// External dependencies
use ring::{digest, hkdf, hmac};
use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use crate::error::CryptoError;
use rand::{Rng, thread_rng};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key(Vec<u8>);

impl Key {
    pub fn new_random() -> Self {
        let mut rng = rand::thread_rng();
        let mut key = [0u8; 16];
        rng.fill(&mut key);
        Key(key.to_vec())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// A custom KeyType that tells Ring's HKDF to produce a 16-byte output.
struct Key16;

impl hkdf::KeyType for Key16 {
    fn len(&self) -> usize {
        16
    }
}

fn pad_message(message: &[u8], target_length: usize) -> Vec<u8> {
    let mut padded = message.to_vec();
    if padded.len() < target_length {
        padded.resize(target_length, 0);
    }
    padded
}


/// Encrypt a padded message using AES-GCM encryption.
///
/// # Arguments
/// * `key` - The encryption key
/// * `message` - The message to encrypt
/// * `padding_size` - The padding size for the message
///
/// # Returns
/// The encrypted message as a byte vector, or an error if encryption fails.
pub fn encrypt(key: &[u8], message: &[u8], padding_size: usize) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes128Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let nonce_bytes = rand::thread_rng().gen::<[u8; 12]>();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut buffer = pad_message(message, padding_size);
    cipher
        .encrypt_in_place(nonce, b"", &mut buffer)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    Ok([nonce.as_slice(), buffer.as_slice()].concat())
}

/// Decrypt a ciphertext encrypted with AES-GCM.
///
/// # Arguments
/// * `key` - The encryption key
/// * `ciphertext` - The encrypted message
///
/// # Returns
/// The decrypted message as a byte vector, or an error if decryption fails.
pub fn decrypt(key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < 12 {
        return Err(CryptoError::DecryptionFailed);
    }
    let cipher = Aes128Gcm::new_from_slice(key)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    let (nonce, ciphertext) = ciphertext.split_at(12);
    let nonce = Nonce::from_slice(nonce);
    let mut buffer = Vec::from(ciphertext);
    cipher
        .decrypt_in_place(nonce, b"", &mut buffer)
        .map_err(|_| CryptoError::DecryptionFailed)?;
    Ok(buffer)
}