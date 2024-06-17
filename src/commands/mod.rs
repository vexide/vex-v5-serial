use thiserror::Error;

use crate::{devices::device::Device, v5::J2000_EPOCH};

pub mod file;

pub trait Command {
    type Error;
    type Response;
    async fn execute(&mut self, device: &mut Device) -> Result<Self::Response, Self::Error>;
}

#[derive(Error, Debug)]
pub enum EncodeStringError {
    #[error("String bytes are too long")]
    StringTooLong,
}

pub(crate) fn encode_string<const MAX_LENGTH: u8>(string: impl AsRef<str>) -> Result<Vec<u8>, EncodeStringError> {
    let string = string.as_ref().as_bytes();
    if string.len() > MAX_LENGTH as usize {
        return Err(EncodeStringError::StringTooLong);
    } else {
        Ok(string.to_vec())
    }
}

pub(crate) fn j2000_timestamp() -> u32 {
    (chrono::Utc::now().timestamp() - J2000_EPOCH as i64) as u32
}