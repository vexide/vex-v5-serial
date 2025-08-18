use std::fmt;

use crate::decode::{Decode, DecodeError};
use crate::encode::{Encode, EncodeError};

/// Variable-width u16 type.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarU16(u16);

impl VarU16 {
    /// Creates a new variable length u16.
    ///
    /// # Panics
    ///
    /// Panics if the value is too large to be encoded as a variable length u16.
    pub fn new(value: u16) -> Self {
        Self::try_new(value).expect("Value too large for variable-length u16")
    }

    /// Creates a new variable length u16.
    pub const fn try_new(value: u16) -> Result<Self, VarU16SizeError> {
        if value > (u16::MAX >> 1) {
            Err(VarU16SizeError(value))
        } else {
            Ok(Self(value))
        }
    }

    pub fn into_inner(self) -> u16 {
        self.0
    }

    /// Check if the variable length u16 will be wide from the first byte.
    pub fn check_wide(first: u8) -> bool {
        first > (u8::MAX >> 1) as _
    }
}
impl Encode for VarU16 {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        if self.0 > (u16::MAX >> 1) {
            return Err(EncodeError::VarShortTooLarge);
        }

        if self.0 > (u8::MAX >> 1) as _ {
            let first = (self.0 >> 8) as u8 | 0x80;
            let last = (self.0 & u8::MAX as u16) as u8;
            Ok([first, last].to_vec())
        } else {
            let val = self.0 as u8;
            Ok(vec![val])
        }
    }
}
impl Decode for VarU16 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let first = u8::decode(&mut data)?;
        let wide = first & (1 << 7) != 0;

        if wide {
            let last = u8::decode(&mut data)?;
            let both = [first & u8::MAX >> 1, last];
            Ok(Self(u16::from_be_bytes(both)))
        } else {
            Ok(Self(first as u16))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VarU16SizeError(u16);

impl fmt::Display for VarU16SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "value {} cannot fit in a variable-length u16", self.0)
    }
}

impl std::error::Error for VarU16SizeError {
    fn description(&self) -> &str {
        "value too large for variable-length u16"
    }
}

#[cfg(test)]
mod tests {
    use crate::{decode::Decode, encode::Encode, varint::VarU16};

    #[test]
    fn wide() {
        // A value that will be encoded as a wide variable length u16.
        const VAL: u16 = 0xF00;
        const ENCODED: [u8; 2] = [0x8f, 0x00];

        let var = super::VarU16::new(VAL);
        assert_eq!(ENCODED.to_vec(), var.encode().unwrap());
        assert_eq!(VAL, VarU16::decode(ENCODED).unwrap().into_inner())
    }

    #[test]
    fn thin() {
        // A value that will be encoded as a thin variable length u16.
        const VAL: u16 = 0x0F;
        const ENCODED: [u8; 1] = [0x0F];

        let var = super::VarU16::new(VAL);
        assert_eq!(ENCODED.to_vec(), var.encode().unwrap());
        assert_eq!(VAL, VarU16::decode(ENCODED).unwrap().into_inner())
    }
}
