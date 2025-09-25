use core::fmt;

use crate::decode::{Decode, DecodeError};
use crate::encode::Encode;

/// A variable-width encoded `u16`.
///
/// `VarU16` encodes a 16-bit unsigned integer in a compact form, where the
/// number of bytes required depends on the value being stored. Small values
/// fit into a single byte, while larger values require two bytes.
///
/// This encoding scheme reserves the most significant bit of the first
/// byte as a flag, indicating the size of the type:
///
/// - If `MSB` is `0`, the value fits in one byte.
/// - If `MSB` is `1`, the value is stored across two bytes.
///
/// # Invariants
///
/// - Encoded values fit into 15 bits (`value <= u16::MAX >> 1`).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarU16 {
    inner: u16,
}

impl VarU16 {
    /// Creates a new [`VarU16`].
    ///
    /// # Panics
    ///
    /// Panics if the given value exceeds the maximum encodable range
    /// (`value > u16::MAX >> 1`).
    pub fn new(value: u16) -> Self {
        Self::try_new(value).expect("Value too large for variable-length u16")
    }

    /// Tries to create a new [`VarU16`].
    ///
    /// # Errors
    ///
    /// Returns a [`VarU16SizeError`] if the given value exceeds the
    /// maximum encodable range (`value > u16::MAX >> 1`).
    pub const fn try_new(value: u16) -> Result<Self, VarU16SizeError> {
        if value > (u16::MAX >> 1) {
            Err(VarU16SizeError { value })
        } else {
            Ok(Self { inner: value })
        }
    }

    /// Returns the inner raw `u16` value.
    pub fn into_inner(self) -> u16 {
        self.inner
    }

    /// Checks whether the given first byte indicates a wide (two-byte) value.
    pub fn check_wide(first: u8) -> bool {
        first > (u8::MAX >> 1) as _
    }
}

impl Encode for VarU16 {
    fn size(&self) -> usize {
        if self.inner > (u8::MAX >> 1) as _ {
            2
        } else {
            1
        }
    }

    fn encode(&self, data: &mut [u8]) {
        if self.inner > (u8::MAX >> 1) as _ {
            data[0] = (self.inner >> 8) as u8 | 0x80;
            data[1] = (self.inner & u8::MAX as u16) as u8;
        } else {
            data[0] = self.inner as u8;
        }
    }
}

impl Decode for VarU16 {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let first = u8::decode(data)?;
        let wide = first & (1 << 7) != 0;

        Ok(Self {
            inner: if wide {
                let last = u8::decode(data)?;
                let both = [first & u8::MAX >> 1, last];
                u16::from_be_bytes(both)
            } else {
                first as u16
            },
        })
    }
}

/// Returned when a [`VarU16`] cannot fit the specified value.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VarU16SizeError {
    pub value: u16,
}

impl fmt::Display for VarU16SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "value {} cannot fit in a variable-length u16",
            self.value
        )
    }
}

impl core::error::Error for VarU16SizeError {
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
        const EXPECTED_ENCODING: [u8; 2] = [0x8f, 0x00];

        let mut buf = [0; 2];

        let var = VarU16::new(VAL);
        var.encode(&mut buf);

        assert_eq!(EXPECTED_ENCODING, buf);
        assert_eq!(
            VAL,
            VarU16::decode(&mut EXPECTED_ENCODING.as_slice())
                .unwrap()
                .into_inner()
        )
    }

    #[test]
    fn thin() {
        // A value that will be encoded as a thin variable length u16.
        const VAL: u16 = 0x0F;
        const EXPECTED_ENCODING: [u8; 1] = [0x0F];

        let mut buf = [0; 1];

        let var = VarU16::new(VAL);
        var.encode(&mut buf);

        assert_eq!(EXPECTED_ENCODING, buf);
        assert_eq!(
            VAL,
            VarU16::decode(&mut EXPECTED_ENCODING.as_slice())
                .unwrap()
                .into_inner()
        )
    }
}
