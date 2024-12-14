use std::fmt::Display;

use crate::{
    decode::{Decode, DecodeError, SizedDecode},
    encode::{Encode, EncodeError},
};

/// A string with a maximum capacity of `len <= N`.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct FixedString<const N: usize>(String);

impl<const N: usize> FixedString<N> {
    pub fn new(string: String) -> Result<Self, EncodeError> {
        if string.as_bytes().len() > N {
            return Err(EncodeError::StringTooLong);
        }

        Ok(Self(string))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const N: usize> Display for FixedString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const N: usize> Encode for FixedString<N> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        self.0.encode()
    }
}

impl<const N: usize> Decode for FixedString<N> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        Ok(Self(String::sized_decode(data, N as u16)?))
    }
}

impl Encode for String {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut bytes = self.as_bytes().to_vec();
        bytes.push(0);
        Ok(bytes)
    }
}

impl SizedDecode for String {
    fn sized_decode(data: impl IntoIterator<Item = u8>, size: u16) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let max_size = size as _;
        let mut data = data.into_iter();

        let mut utf8 = vec![0u8; max_size];
        for (i, string_byte) in utf8.iter_mut().enumerate() {
            let byte = u8::decode(&mut data)?;
            if i == max_size {
                if byte != 0 {
                    return Err(DecodeError::UnterminatedString);
                }
                break;
            }
            if byte == 0 {
                break;
            }

            *string_byte = byte;
        }

        Ok(String::from_utf8(utf8.to_vec())?)
    }
}
