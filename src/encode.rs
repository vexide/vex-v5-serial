use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("String bytes are too long")]
    StringTooLong,
    #[error("Value too large for variable length u16")]
    VarShortTooLarge,
}

/// A trait that allows for encoding a structure into a byte sequence.
pub trait Encode {
    /// Encodes a structure into a byte sequence.
    fn encode(&self) -> Result<Vec<u8>, EncodeError>;
    fn into_encoded(self) -> Result<Vec<u8>, EncodeError>
    where
        Self: Sized,
    {
        self.encode()
    }
    
    // TODO: Come up with a better solution than this. This is a band-aid fix to allow encoding crc checksums
    fn extended() -> bool {
        false
    }
}
impl Encode for () {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(Vec::new())
    }
}
impl Encode for Vec<u8> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.clone())
    }
}