use std::{mem::MaybeUninit, str::Utf8Error};
use thiserror::Error;

use crate::string::FixedStringSizeError;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DecodeError {
    #[error("Packet too short")]
    UnexpectedEnd,

    #[error("Could not decode byte with unexpected value. Found {value:x}, expected one of: {expected:x?}")]
    UnexpectedValue { value: u8, expected: &'static [u8] },

    #[error("Invalid response header")]
    InvalidHeader,

    #[error("String ran past expected nul terminator")]
    UnterminatedString,

    #[error(transparent)]
    FixedStringSizeError(#[from] FixedStringSizeError),

    #[error("String contained invalid UTF-8: {0}")]
    InvalidStringContents(#[from] Utf8Error),
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

pub trait Decode {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

pub trait DecodeWithLength {
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
                    let bytes = data.get(..size_of::<Self>()).ok_or_else(|| DecodeError::UnexpectedEnd)?;
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

        Ok(unsafe { std::mem::transmute_copy::<_, [T; N]>(&arr) })
    }
}