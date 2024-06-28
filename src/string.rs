use std::fmt::Display;

use crate::{
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
};

#[derive(Debug, Clone)]
pub struct DynamicVarLengthString(pub String, pub usize);
impl DynamicVarLengthString {
    pub fn new(string: String, max_size: usize) -> Result<Self, EncodeError> {
        if string.len() > max_size {
            return Err(EncodeError::StringTooLong);
        }

        Ok(Self(string, max_size))
    }
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn decode_with_max_size(
        data: impl IntoIterator<Item = u8>,
        max_size: usize,
    ) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let mut string_bytes = vec![0u8; max_size];
        for (i, string_byte) in string_bytes.iter_mut().enumerate() {
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
        Ok(Self(String::from_utf8(string_bytes.to_vec())?, max_size))
    }
}

#[derive(Debug, Clone)]
pub struct VarLengthString<const MAX_LEN: usize>(String);
impl<const MAX_LEN: usize> VarLengthString<MAX_LEN> {
    pub fn new(string: String) -> Result<Self, EncodeError> {
        if string.as_bytes().len() > MAX_LEN {
            return Err(EncodeError::StringTooLong);
        }

        Ok(Self(string))
    }
}
impl<const MAX_LEN: usize> Encode for VarLengthString<MAX_LEN> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut bytes = self.0.as_bytes().to_vec();
        bytes.push(0);
        Ok(bytes)
    }
}
impl<const MAX_LEN: usize> Decode for VarLengthString<MAX_LEN> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let mut string_bytes = [0u8; MAX_LEN];
        for (i, string_byte) in string_bytes.iter_mut().enumerate() {
            let byte = u8::decode(&mut data)?;
            if i == MAX_LEN {
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

        Ok(Self(String::from_utf8(string_bytes.to_vec())?))
    }
}
/// A null-terminated fixed length string.
/// Once encoded, the size will be `LEN + 1` bytes.
#[derive(Debug, Clone)]
pub struct FixedLengthString<const LEN: usize>(String);
impl<const LEN: usize> FixedLengthString<LEN> {
    pub fn new(string: String) -> Result<Self, EncodeError> {
        if string.as_bytes().len() > LEN {
            return Err(EncodeError::StringTooLong);
        }

        Ok(Self(string))
    }
}
impl<const LEN: usize> Encode for FixedLengthString<LEN> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = [0u8; LEN];

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
impl<const LEN: usize> Decode for FixedLengthString<LEN> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let string_bytes: [u8; LEN] = Decode::decode(&mut data)?;
        let terminator = u8::decode(&mut data)?;
        if terminator != 0 {
            Err(DecodeError::UnterminatedString)
        } else {
            Ok(Self(String::from_utf8(string_bytes.to_vec())?))
        }
    }
}
impl<const LEN: usize> Display for FixedLengthString<LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{decode::Decode, encode::Encode};

    use super::FixedLengthString;

    #[test]
    #[should_panic]
    fn invalid_fixed_length_string() {
        let _ = FixedLengthString::<4>::new("hello world".to_string()).unwrap();
    }
    #[test]
    fn fixed_length_string() {
        let string = FixedLengthString::<10>::new("helloworld".to_string()).unwrap();
        let encoded = string.encode().unwrap();
        // 10 bytes for the string, 1 byte for the null terminator.
        assert_eq!(encoded.len(), 10 + 1);

        let bytes = b"helloworld\0".to_vec();
        assert_eq!(encoded, bytes);
        let decoded_string = FixedLengthString::<10>::decode(bytes).unwrap();

        assert_eq!(decoded_string.0, "helloworld".to_string());
    }
}