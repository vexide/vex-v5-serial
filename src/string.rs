use core::fmt;
use std::{ffi::CStr, fmt::Display, str::FromStr};

use crate::{
    decode::{Decode, DecodeError, DecodeWithLength},
    encode::Encode,
};

/// A string with a maximum capacity of `len <= N`.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
pub struct FixedString<const N: usize>(String);

impl<const N: usize> FixedString<N> {
    pub fn new(string: impl AsRef<str>) -> Result<Self, FixedStringSizeError> {
        let string = string.as_ref().to_string();
        let string_len = string.as_bytes().len();

        if string_len > N {
            return Err(FixedStringSizeError {
                input_len: string_len,
                max_string_len: N,
            });
        }

        Ok(Self(string))
    }

    /// # Safety
    ///
    /// This function is unsafe because it does not check if the string is longer than the maximum length.
    pub unsafe fn new_unchecked(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_string())
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const N: usize> TryFrom<&str> for FixedString<N> {
    type Error = FixedStringSizeError;

    fn try_from(value: &str) -> Result<FixedString<N>, FixedStringSizeError> {
        Self::new(value.to_string())
    }
}

impl<const N: usize> FromStr for FixedString<N> {
    type Err = FixedStringSizeError;

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
    fn size(&self) -> usize {
        N + 1
    }

    fn encode(&self, data: &mut [u8]) {
        let data_len = self.0.len();

        data[..data_len].copy_from_slice(self.0.as_bytes());
        data[data_len + 1] = 0; // Null terminator
    }
}

impl<const N: usize> Decode for FixedString<N> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self(String::decode_with_len(data, N)?))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FixedStringSizeError {
    input_len: usize,
    max_string_len: usize,
}

impl fmt::Display for FixedStringSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "string with len {} exceeds the maximum length of FixedString<{}>",
            self.input_len, self.max_string_len
        )
    }
}

impl std::error::Error for FixedStringSizeError {
    fn description(&self) -> &str {
        "string exceeds the maximum length of FixedString"
    }
}

impl Encode for &str {
    fn size(&self) -> usize {
        self.len() + 1 // +1 for null terminator
    }

    fn encode(&self, data: &mut [u8]) {
        let bytes = self.as_bytes();

        data[..bytes.len()].copy_from_slice(bytes);
        data[bytes.len()] = 0;
    }
}

impl Encode for String {
    fn size(&self) -> usize {
        self.as_str().size()
    }

    fn encode(&self, data: &mut [u8]) {
        self.as_str().encode(data)
    }
}

impl DecodeWithLength for String {
    fn decode_with_len(data: &mut &[u8], len: usize) -> Result<Self, DecodeError> {
        let max_size = len as _;

        let mut utf8 = vec![0u8; max_size];
        for (i, string_byte) in utf8.iter_mut().enumerate() {
            let byte = u8::decode(data)?;

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
