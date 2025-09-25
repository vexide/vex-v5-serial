use core::{
    borrow::{Borrow, BorrowMut},
    ffi::CStr,
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};
use core::{fmt, str};

use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec,
};

use crate::{
    decode::{Decode, DecodeError, DecodeErrorKind, DecodeWithLength},
    encode::Encode,
};

/// A UTF-8 string with a fixed maximum capacity of `N` bytes.
///
/// `FixedString<N>` stores string data inline, backed by a `[u8; N]` buffer.
/// Unlike [`String`], its capacity is fixed at compile time. The actual
/// string length may be smaller than `N`, but may never exceed it.
///
/// # Invariants
///
/// - Contents are always valid UTF-8.
/// - The inner string must satisfy `bytes.len() <= N`.
/// - All bytes past the end of the string are zeroed.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
pub struct FixedString<const N: usize>([u8; N]);

impl<const N: usize> FixedString<N> {
    /// Creates a new [`FixedString`] from the given string slice.
    ///
    /// Fails if the input string is larger than the fixed capacity `N`.
    ///
    /// # Errors
    ///
    /// Returns [`FixedStringSizeError`] if the string’s UTF-8 byte length
    /// exceeds `N`.
    pub fn new(s: impl AsRef<str>) -> Result<Self, FixedStringSizeError> {
        let size = s.as_ref().as_bytes().len();

        if size > N {
            return Err(FixedStringSizeError {
                input_size: size,
                max_size: N,
            });
        }

        // SAFETY: We have verified that s.as_bytes().len() <= N above.
        Ok(unsafe { Self::new_unchecked(s) })
    }

    /// Creates a new [`FixedString`] without checking the size.
    ///
    /// If the input string is longer than `N` bytes, it will be truncated
    /// to fit into the buffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `s` is valid UTF-8 and that truncation
    /// does not violate invariants of how the string is later used.
    ///
    /// Normally you should prefer [`FixedString::new`], which enforces the
    /// size bound at runtime.
    pub unsafe fn new_unchecked(s: impl AsRef<str>) -> Self {
        let s = s.as_ref();
        let bytes = s.as_bytes();
        let len = bytes.len().min(N); // truncate if necessary

        let mut buf = [0; N];
        buf[..len].copy_from_slice(&bytes[..len]);

        Self(buf)
    }

    /// Extracts a string slice containing this string's contents.
    pub fn as_str(&self) -> &str {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(N);

        // SAFETY: Construction guarantees valid UTF-8 up to `len`.
        unsafe { str::from_utf8_unchecked(&self.0[..len]) }
    }

    /// Converts a `FixedString` into a mutable string slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(N);

        unsafe { str::from_utf8_unchecked_mut(&mut self.0[..len]) }
    }
}

impl<const N: usize> Deref for FixedString<N> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> DerefMut for FixedString<N> {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const N: usize> Default for FixedString<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> AsRef<str> for FixedString<N> {
    fn as_ref(&self) -> &str {
        self
    }
}

impl<const N: usize> AsMut<str> for FixedString<N> {
    fn as_mut(&mut self) -> &mut str {
        self
    }
}

impl<const N: usize> Borrow<str> for FixedString<N> {
    #[inline]
    fn borrow(&self) -> &str {
        &self[..]
    }
}

impl<const N: usize> BorrowMut<str> for FixedString<N> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut str {
        &mut self[..]
    }
}

impl<const N: usize> AsRef<[u8]> for FixedString<N> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<const N: usize> TryFrom<&str> for FixedString<N> {
    type Error = FixedStringSizeError;

    fn try_from(value: &str) -> Result<FixedString<N>, FixedStringSizeError> {
        Self::new(value)
    }
}

impl<const N: usize> TryFrom<&mut str> for FixedString<N> {
    type Error = FixedStringSizeError;

    fn try_from(value: &mut str) -> Result<FixedString<N>, FixedStringSizeError> {
        Self::new(value)
    }
}

impl<const N: usize> FromStr for FixedString<N> {
    type Err = FixedStringSizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

impl<const N: usize> Display for FixedString<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl<const N: usize> Encode for FixedString<N> {
    fn size(&self) -> usize {
        N + 1
    }

    fn encode(&self, data: &mut [u8]) {
        let data_len = self.0.len();

        data[..data_len].copy_from_slice(self.as_bytes());
        data[data_len + 1] = 0; // Null terminator
    }
}

impl<const N: usize> Decode for FixedString<N> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(unsafe { Self::new_unchecked(String::decode_with_len(data, N)?) })
    }
}

/// Returned when a [`FixedString`] cannot fit the specified string.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FixedStringSizeError {
    pub input_size: usize,
    pub max_size: usize,
}

impl fmt::Display for FixedStringSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "string with size {} exceeds the maximum size of FixedString<{}>",
            self.input_size, self.max_size
        )
    }
}

impl core::error::Error for FixedStringSizeError {
    fn description(&self) -> &str {
        "string exceeds the maximum size of FixedString"
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
                    return Err(DecodeError::new::<Self>(
                        DecodeErrorKind::UnterminatedString,
                    ));
                }
                break;
            }
            if byte == 0 {
                break;
            }

            *string_byte = byte;
        }

        let cstr = CStr::from_bytes_until_nul(&utf8)
            .map_err(|_| DecodeError::new::<Self>(DecodeErrorKind::UnterminatedString))?;

        Ok(cstr
            .to_str()
            .map_err(|e| DecodeError::new::<Self>(e.into()))?
            .to_owned())
    }
}
