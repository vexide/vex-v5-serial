use std::string::FromUtf8Error;

use thiserror::Error;

use crate::v5::J2000_EPOCH;

pub mod capture;
pub mod cdc;
pub mod cdc2;
pub mod controller;
pub mod dash;
pub mod device;
pub mod factory;
pub mod file;
pub mod kv;
pub mod log;
pub mod radio;
pub mod slot;
pub mod system;

#[repr(transparent)]
pub struct VarU16(u16);
impl VarU16 {
    /// Creates a new variable length u16.
    /// # Panics
    /// Panics if the value is too large to be encoded as a variable length u16.
    pub fn new(val: u16) -> Self {
        if val > (u16::MAX >> 1) {
            panic!("Value too large for variable length u16");
        }
        Self(val)
    }
}
impl Encode for VarU16 {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        if self.0 > (u16::MAX >> 1) {
            return Err(EncodeError::VarShortTooLarge);
        }

        if self.0 > (u8::MAX >> 1) as _ {
            let mut val = self.0.to_le_bytes();
            val[0] |= 1 << 7;
            Ok(val.to_vec())
        } else {
            let val = self.0 as u8;
            Ok(vec![val])
        }
    }
}

pub(crate) fn j2000_timestamp() -> u32 {
    (chrono::Utc::now().timestamp() - J2000_EPOCH as i64) as u32
}

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

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("String bytes are too long")]
    StringTooLong,
    #[error("Value too large for variable length u16")]
    VarShortTooLarge,
}

/// A trait that allows for encoding a structure into a byte sequence.
pub trait Encode {
    /// Encodes a structure into a byte sequence.
    fn encode(&self) -> Result<Vec<u8>, EncodeError>;
    fn into_encoded(self) -> Result<Vec<u8>, EncodeError>
    where
        Self: Sized,
    {
        self.encode()
    }
}
impl Encode for () {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(Vec::new())
    }
}
impl Encode for Vec<u8> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.clone())
    }
}

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Packet too short")]
    PacketTooShort,
    #[error("Invalid response header")]
    InvalidHeader,
    #[error("String ran past expected nul terminator")]
    UnterminatedString,
    #[error("String contained invalid UTF-8: {0}")]
    InvalidStringContents(#[from] FromUtf8Error),
    #[error("Could not decode byte with unexpected value")]
    UnexpectedValue
}

pub trait Decode {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized;
}
impl Decode for () {
    fn decode(_data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        Ok(())
    }
}
impl Decode for u8 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        data.next().ok_or(DecodeError::PacketTooShort)
    }
}
impl Decode for i8 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        // This is just a tad silly, but id rather not transmute
        data.next()
            .map(|byte| i8::from_le_bytes([byte]))
            .ok_or(DecodeError::PacketTooShort)
    }
}
impl Decode for u16 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(u16::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl Decode for i16 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(i16::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl Decode for u32 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(u32::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl Decode for i32 {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        Ok(i32::from_le_bytes(Decode::decode(&mut data)?))
    }
}
impl<D: Decode> Decode for Option<D> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        Ok(D::decode(data).map(|decoded| Some(decoded)).unwrap_or(None))
    }
}
impl<D: Decode, const N: usize> Decode for [D; N] {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        std::array::try_from_fn(move |_| D::decode(&mut data))
    }
}
impl Decode for Vec<u8> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        Ok(data.into_iter().collect())
    }
}

/// Device-bound Communications Packet
///
/// This structure encodes a data payload and ID that is intended to be sent from
/// a host machine to a V5 device over the serial protocol. This is typically done
/// through either a [`CdcCommandPacket`] or a [`Cdc2CommandPacket`].
pub struct DeviceBoundPacket<P: Encode, const ID: u8> {
    /// Device-bound Packet Header
    ///
    /// This must be `Self::HEADER` or `[0xC9, 0x36, 0xB8, 0x47]`.
    header: [u8; 4],

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the device.
    payload: P,
}
impl<P: Encode, const ID: u8> Encode for DeviceBoundPacket<P, ID> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&self.header);
        encoded.push(ID);

        let size = VarU16::new(self.payload.encode()?.len() as u16);
        encoded.extend(size.encode()?);

        encoded.extend_from_slice(&self.payload.encode()?);
        Ok(encoded)
    }
}

impl<P: Encode, const ID: u8> DeviceBoundPacket<P, ID> {
    /// Header byte sequence used for all device-bound packets.
    pub const HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: Self::HEADER,
            payload,
        }
    }
}

/// Host-bound Communications Packet
///
/// This structure encodes a data payload and ID that is intended to be sent from
/// a V5 device to a host machine over the serial protocol. This is typically done
/// through either a [`CdcReplyPacket`] or a [`Cdc2ReplyPacket`].
pub struct HostBoundPacket<P: Decode, const ID: u8> {
    /// Host-bound Packet Header
    ///
    /// This must be `Self::HEADER` or `[0xAA, 0x55]`.
    header: [u8; 2],

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    payload: P,
}

impl<P: Decode, const ID: u8> HostBoundPacket<P, ID> {
    /// Header byte sequence used for all host-bound packets.
    pub const HEADER: [u8; 2] = [0xAA, 0x55];

    /// Creates a new host-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: Self::HEADER,
            payload,
        }
    }
}
impl<P: Decode, const ID: u8> Decode for HostBoundPacket<P, ID> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let header = Decode::decode(&mut data)?;
        if header != Self::HEADER {
            return Err(DecodeError::InvalidHeader);
        }
        let payload = P::decode(data)?;

        Ok(Self { header, payload })
    }
}

pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub beta: u8,
}
impl Encode for Version {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.major, self.minor, self.build, self.beta])
    }
}
impl Decode for Version {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let major = u8::decode(&mut data)?;
        let minor = u8::decode(&mut data)?;
        let build = u8::decode(&mut data)?;
        let beta = u8::decode(&mut data)?;
        Ok(Self {
            major,
            minor,
            build,
            beta,
        })
    }
}
