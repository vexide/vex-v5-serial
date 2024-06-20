use thiserror::Error;
use std::string::FromUtf8Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Packet too short")]
    PacketTooShort,
    #[error("Invalid response header")]
    InvalidHeader,
    #[error("String ran past expected nul terminator")]
    UnterminatedString,
    #[error("String contained invalid UTF-8: {0}")]
    InvalidStringContents(#[from] FromUtf8Error),
    #[error("Could not decode byte with unexpected value")]
    UnexpectedValue,
}

pub trait Decode {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized;
}
impl Decode for () {
    fn decode(_data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        Ok(())
    }
}
impl Decode for u8 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        data.next().ok_or(DecodeError::PacketTooShort)
    }
}
impl Decode for i8 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        // This is just a tad silly, but id rather not transmute
        data.next()
            .map(|byte| i8::from_le_bytes([byte]))
            .ok_or(DecodeError::PacketTooShort)
    }
}
impl Decode for u16 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(u16::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl Decode for i16 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(i16::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl Decode for u32 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(u32::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl Decode for i32 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(i32::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl<D: Decode> Decode for Option<D> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        Ok(D::decode(data).map(|decoded| Some(decoded)).unwrap_or(None))
    }
}
impl<D: Decode, const N: usize> Decode for [D; N] {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        std::array::try_from_fn(move |_| D::decode(&mut data))
    }
}