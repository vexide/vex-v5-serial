use alloc::vec::Vec;
use core::{mem::MaybeUninit, str::Utf8Error};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub struct DecodeError {
    kind: DecodeErrorKind,
    type_name: &'static str,
}

impl DecodeError {
    pub fn new<T>(kind: DecodeErrorKind) -> Self {
        Self {
            kind,
            type_name: core::any::type_name::<T>(),
        }
    }

    pub const fn kind(&self) -> DecodeErrorKind {
        self.kind
    }
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Failed to decode {}: {}", self.type_name, self.kind)
    }
}

#[derive(Error, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeErrorKind {
    #[error("Packet was too short.")]
    UnexpectedEnd,

    #[error(
        "Could not decode {name} with unexpected byte. Found {value:x}, expected one of: {expected:x?}."
    )]
    UnexpectedByte {
        name: &'static str,
        value: u8,
        expected: &'static [u8],
    },

    #[error(
        "CRC16 checksum mismatch. Found {value:x}, expected {expected:x}."
    )]
    Checksum {
        value: u16,
        expected: u16,
    },

    #[error("Packet did not have a valid header sequence.")]
    InvalidHeader,

    #[error("String ran past expected null terminator.")]
    UnterminatedString,

    #[error(transparent)]
    Utf8Error(#[from] Utf8Error),
}

impl<T: Decode> DecodeWithLength for Vec<T> {
    fn decode_with_len(data: &mut &[u8], len: usize) -> Result<Self, DecodeError> {
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::decode(data)?);
        }
        Ok(vec)
    }
}

/// A type that can be reconstructed (decoded) from a raw sequence of bytes.
///
/// Implementors of this trait define how to parse their binary representation
/// from an input buffer. The input slice will be advanced by the number of bytes
/// successfully consumed during decoding.
pub trait Decode {
    /// Attempts to decode `Self` from the beginning of the provided byte slice.
    ///
    /// On success, returns the decoded value and advances `data` by the number
    /// of bytes consumed. On failure, returns a [`DecodeError`].
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] if the input is malformed or insufficient
    /// to decode a complete value of this type.
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

/// A type that can be decoded from a sequence of bytes, given an indicator of
/// the number of items contained within the type.
///
/// This is primarily intended for collection-like types (e.g. [`Vec`]) whose
/// number of elements must be known before decoding can proceed. The caller
/// provides `len` as the number of items expected to be decoded.
///
/// Like [`Decode`], the input slice will be advanced by the number of bytes
/// successfully consumed.
pub trait DecodeWithLength {
    /// Attempts to decode `Self` from the provided byte slice, consuming exactly
    /// `len` items.
    ///
    /// On success, returns the decoded value and advances `data` by the number
    /// of bytes consumed. On failure, returns a [`DecodeError`].
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] if the input is malformed or insufficient
    /// to decode a complete value of this type.
    fn decode_with_len(data: &mut &[u8], len: usize) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

impl Decode for () {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(())
    }
}

macro_rules! impl_decode_for_primitive {
    ($($t:ty),*) => {
        $(
            impl Decode for $t {
                fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
                    let bytes = data.get(..size_of::<Self>()).ok_or_else(|| DecodeError::new::<Self>(DecodeErrorKind::UnexpectedEnd))?;
                    *data = &data[size_of::<Self>()..];
                    Ok(Self::from_le_bytes(bytes.try_into().unwrap()))
                }
            }
        )*
    };
}

impl_decode_for_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

// TODO: Switch to try_from_fn and/or array::try_map once stabilized
impl<const N: usize, T: Decode> Decode for [T; N] {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let mut arr: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for i in 0..N {
            arr[i] = MaybeUninit::new(T::decode(data)?);
        }

        Ok(unsafe { core::mem::transmute_copy::<_, [T; N]>(&arr) })
    }
}
