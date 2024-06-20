use crate::{
    encode::{Encode, EncodeError},
    decode::{Decode, DecodeError},
};

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
        for i in 0..=max_size {
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

            string_bytes[i] = byte;
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
        for i in 0..=MAX_LEN {
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

            string_bytes[i] = byte;
        }

        Ok(Self(String::from_utf8(string_bytes.to_vec())?))
    }
}
/// A null-terminated fixed length string.
/// Once encoded, the size will be `LEN + 1` bytes.
#[derive(Debug, Clone)]
pub struct TerminatedFixedLengthString<const LEN: usize>(String);
impl<const LEN: usize> TerminatedFixedLengthString<LEN> {
    pub fn new(string: String) -> Result<Self, EncodeError> {
        if string.as_bytes().len() > LEN {
            return Err(EncodeError::StringTooLong);
        }

        Ok(Self(string))
    }
}
impl<const LEN: usize> Encode for TerminatedFixedLengthString<LEN> {
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
impl<const LEN: usize> Decode for TerminatedFixedLengthString<LEN> {
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

pub struct UnterminatedFixedLengthString<const LEN: usize>([u8; LEN]);
impl<const LEN: usize> UnterminatedFixedLengthString<LEN> {
    pub fn new(string: String) -> Result<Self, EncodeError> {
        let mut encoded = [0u8; LEN];

        let string_bytes = string.into_bytes();
        if string_bytes.len() > encoded.len() {
            return Err(EncodeError::StringTooLong);
        }

        encoded[..string_bytes.len()].copy_from_slice(&string_bytes);

        Ok(Self(encoded))
    }
}
impl Encode for UnterminatedFixedLengthString<23> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.0.to_vec())
    }
}