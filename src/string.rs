use std::{ffi::CStr, fmt::Display, str::FromStr};

use crate::{
    decode::{Decode, DecodeError, SizedDecode},
    encode::{Encode, EncodeError},
};

/// A string with a maximum capacity of `len <= N`.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
pub struct FixedString<const N: usize>(String);

impl<const N: usize> FixedString<N> {
    pub fn new(string: impl AsRef<str>) -> Result<Self, EncodeError> {
        let string = string.as_ref().to_string();

        if string.as_bytes().len() > N {
            return Err(EncodeError::StringTooLong);
        }

        Ok(Self(string))
    }

    /// # Safety
    /// 
    /// This function is unsafe because it does not check if the string is longer than the maximum length.
    pub unsafe fn new_unchecked(string: String) -> Self {
        Self(string)
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const N: usize> TryFrom<&str> for FixedString<N> {
    type Error = EncodeError;

    fn try_from(value: &str) -> Result<FixedString<N>, EncodeError> {
        Self::new(value.to_string())
    }
}

impl<const N: usize> FromStr for FixedString<N> {
    type Err = EncodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

impl<const N: usize> AsRef<str> for FixedString<N> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<const N: usize> Display for FixedString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const N: usize> Encode for FixedString<N> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = [0u8; N];

        let string_bytes = self.0.clone().into_bytes();
        if string_bytes.len() > encoded.len() {
            return Err(EncodeError::StringTooLong);
        }

        encoded[..string_bytes.len()].copy_from_slice(&string_bytes);
        let mut encoded = encoded.to_vec();
        encoded.push(0);
        Ok(encoded)
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

        let cstr =
            CStr::from_bytes_until_nul(&utf8).map_err(|_| DecodeError::UnterminatedString)?;

        Ok(cstr.to_str()?.to_owned())
    }
}
