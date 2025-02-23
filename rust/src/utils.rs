// External dependencies
use ring::{digest, hkdf, hmac};
use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use crate::error::CryptoError;
use rand::Rng;

/// A custom KeyType that tells Ring's HKDF to produce a 16-byte output.
struct Key16;

impl hkdf::KeyType for Key16 {
    fn len(&self) -> usize {
        16
    }
}

/// Key Derivation Function (KDF) that derives a 16-byte key from a 128-bit input key and a string.
///
/// Uses HKDF-SHA256 with a fixed salt to derive the key.
///
/// # Arguments
/// * `key` - The input key bytes to derive from (must be 16 bytes long)
/// * `input` - A string to mix into the derivation (serves as the HKDF "info")
///
/// # Returns
/// * `Ok(Vec<u8>)` - The derived 16-byte key
/// * `Err(MycoError)` - If HKDF expansion or fill fails, or if the input key length is invalid
pub fn kdf(key: &[u8], input: &str) -> Result<Vec<u8>, CryptoError> {
    // Enforce that the input key is exactly 16 bytes (128 bits).
    if key.len() != 16 {
        return Err(CryptoError::InvalidKeyLength);
    }

    // Compute a fixed salt (here, the SHA-256 digest of a constant string).
    let salt = digest::digest(&digest::SHA256, b"MC-OSAM-Salt");

    // Extract the pseudorandom key (PRK) using Ring's HKDF.
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt.as_ref()).extract(key);

    // Use the provided input (as bytes) as the HKDF "info" parameter.
    let info = [input.as_bytes()];

    // Expand to produce exactly 16 bytes of output.
    let okm = prk
        .expand(&info, Key16)
        .map_err(|_| CryptoError::HkdfExpansionFailed)?;

    let mut result = [0u8; 16];
    okm.fill(&mut result)
        .map_err(|_| CryptoError::HkdfFillFailed)?;

    Ok(result.to_vec())
}

/// Alternative PRF using HMAC-SHA256 directly.
///
/// This version uses HMAC-SHA256 with the provided key (which can be 16 bytes)
/// to produce 16 bytes of pseudorandom output (by truncating the 32-byte tag).
///
/// # Arguments
/// * `key` - The input key bytes (can be 16 bytes or another acceptable length)
/// * `input` - Input bytes to mix into the PRF
///
/// # Returns
/// * `Ok(Vec<u8>)` - 16 bytes of pseudorandom output
/// * `Err(MycoError)` - In case of an error (though HMAC typically does not fail)
pub fn _prf(key: &[u8], input: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let signing_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let tag = hmac::sign(&signing_key, input);
    // Truncate the 32-byte tag to 16 bytes.
    Ok(tag.as_ref()[..16].to_vec())
}

/// Wrapper for the PRF function that returns a fixed number of bytes.
///
/// This function calls the internal `_prf` function (which uses HMAC-SHA256 to produce a 16-byte output)
/// and returns the first `length` bytes of the output.
///
/// # Arguments
/// * `key` - The 128-bit key (16 bytes)
/// * `input` - Input bytes to mix into the PRF
/// * `length` - The number of bytes to return from the PRF output
///
/// # Returns
/// * `Ok(Vec<u8>)` - A byte array containing the first `length` bytes of the PRF output.
/// * `Err(MycoError)` - If the underlying PRF call fails.
///
pub fn prf(key: &[u8], input: &[u8], _max_value: usize) -> Result<Vec<u8>, CryptoError> {
    // Call the underlying PRF function to get 16 bytes (128 bits) of pseudorandom output.
    let prf_output = _prf(key, input)?;
    
    Ok(prf_output)
}

/// A wrapper around the PRF function that returns a fixed number of bytes.
///
/// This function calls the internal `_prf` function (which uses HMAC-SHA256 to produce a 16-byte output)
/// and returns the first `length` bytes of the output.
///
/// # Arguments
/// * `key` - The 128-bit key (16 bytes)
/// * `input` - Input bytes to mix into the PRF
/// * `length` - The number of bytes to return from the PRF output
///
/// # Returns
/// * `Ok(Vec<u8>)` - A byte array containing the first `length` bytes of the PRF output.
/// * `Err(MycoError)` - If the underlying PRF call fails.
///
pub fn prf_fixed_length(key: &[u8], input: &[u8], length: usize) -> Result<Vec<u8>, CryptoError> {
    // Call the underlying PRF function to get 16 bytes (128 bits) of pseudorandom output.
    let prf_output = _prf(key, input)?;
    
    // Return the first `length` bytes
    Ok(prf_output[..length].to_vec())
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