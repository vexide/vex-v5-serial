use std::str::Utf8Error;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DecodeError {
    pub kind: DecodeErrorKind,
    pub decoded_type: &'static str,
}
impl DecodeError {
    pub fn new<Packet>(kind: DecodeErrorKind) -> Self {
        Self { kind, decoded_type: std::any::type_name::<Packet>() }
    }
}
impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DecodeError: {}: {}", self.decoded_type, self.kind)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DecodeErrorKind {
    #[error("Packet too short")]
    PacketTooShort,
    #[error("Invalid response header")]
    InvalidHeader,
    #[error("String ran past expected nul terminator")]
    UnterminatedString,
    #[error("String contained invalid UTF-8: {0}")]
    InvalidStringContents(#[from] Utf8Error),
    #[error("Could not decode byte with unexpected value. Found {value:x}, expected one of: {expected:x?}")]
    UnexpectedValue { value: u8, expected: &'static [u8] },
    #[error("Attempted to decode a choice, but neither choice was successful: left: {left:?}, right: {right:?}")]
    BothChoicesFailed {
        left: Box<DecodeError>,
        right: Box<DecodeError>,
    },
}

pub trait Decode {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized;
}
pub trait SizedDecode {
    fn sized_decode(data: impl IntoIterator<Item = u8>, size: u16) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

impl<T: Decode> SizedDecode for T {
    fn sized_decode(data: impl IntoIterator<Item = u8>, _: u16) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        Decode::decode(data)
    }
}

impl Decode for () {
    fn decode(_data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        Ok(())
    }
}
impl Decode for u8 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        data.next().ok_or(DecodeError::new::<u8>(DecodeErrorKind::PacketTooShort))
    }
}
impl Decode for i8 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        // This is just a tad silly, but id rather not transmute
        data.next()
            .map(|byte| i8::from_le_bytes([byte]))
            .ok_or(DecodeError::new::<i8>(DecodeErrorKind::PacketTooShort))
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
        D::decode(data).map(|decoded| Some(decoded))
    }
}
impl<D: Decode + Default, const N: usize> Decode for [D; N] {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let results: [_; N] = std::array::from_fn(move |_| D::decode(&mut data));
        let mut decoded = Vec::new();
        for result in results.into_iter() {
            match result {
                Ok(d) => decoded.push(d),
                Err(e) => return Err(e),
            }
        }
        let mut decoded_array = std::array::from_fn(|_| D::default());
        decoded_array
            .iter_mut()
            .zip(decoded)
            .for_each(|(a, b)| *a = b);

        Ok(decoded_array)
    }
}

impl<T: Decode> SizedDecode for Vec<T> {
    fn sized_decode(data: impl IntoIterator<Item = u8>, len: u16) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut data = data.into_iter();
        let mut vec = Vec::with_capacity(len as usize);
        for _ in 0..len {
            vec.push(T::decode(&mut data)?);
        }
        Ok(vec)
    }
}
