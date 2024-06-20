use thiserror::Error;

use crate::devices::{device::Device, DeviceError};

pub mod file;

pub trait Command {
    type Output;
    async fn execute(&mut self, device: &mut Device) -> Result<Self::Output, DeviceError>;
}

#[derive(Error, Debug)]
pub enum EncodeStringError {
    #[error("String bytes are too long")]
    StringTooLong,
}

pub(crate) fn encode_string<const MAX_LENGTH: u8>(
    string: impl AsRef<str>,
) -> Result<Vec<u8>, EncodeStringError> {
    let string = string.as_ref().as_bytes();
    if string.len() > MAX_LENGTH as usize {
        Err(EncodeStringError::StringTooLong)
    } else {
        Ok(string.to_vec())
    }
}
