use thiserror::Error;

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PirStatus {
    Success = 0,
    ErrorInvalidArgument = -1,
    ErrorMemory = -2,
    ErrorProcessing = -3,
}

#[derive(Error, Debug)]
pub enum PirError {
    #[error("Invalid argument provided")]
    InvalidArgument,
    #[error("Memory allocation or management error")]
    Memory,
    #[error("Error during request processing")]
    Processing,
    #[error("Invalid UTF-8 in response")]
    Utf8Error,
    #[error("Foreign function interface error")]
    FfiError,
    #[error("Cuckoo Table is full")]
    TableFull,
    #[error("Index out of bounds")]
    IndexOutOfBounds,
}

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key length - must be exactly 16 bytes")]
    InvalidKeyLength,
    #[error("HKDF expansion failed")]
    HkdfExpansionFailed,
    #[error("HKDF fill failed")]
    HkdfFillFailed,
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
}


impl From<PirStatus> for Result<(), PirError> {
    fn from(status: PirStatus) -> Self {
        match status {
            PirStatus::Success => Ok(()),
            PirStatus::ErrorInvalidArgument => Err(PirError::InvalidArgument),
            PirStatus::ErrorMemory => Err(PirError::Memory),
            PirStatus::ErrorProcessing => Err(PirError::Processing),
            _ => Err(PirError::FfiError),
        }
    }
}
