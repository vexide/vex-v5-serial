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

/// Encodes a u16 as an unsigned var short.
pub(crate) fn encode_var_u16(val: u16) -> Result<Vec<u8>, EncodeError> {
    if val > (u16::MAX >> 1) {
        return Err(EncodeError::VarShortTooLarge);
    }

    if val > (u8::MAX >> 1) as _ {
        let mut val = val.to_le_bytes();
        val[0] |= 1 << 7;
        Ok(val.to_vec())
    } else {
        let val = val as u8;
        Ok(vec![val])
    }
}

pub(crate) fn j2000_timestamp() -> u32 {
    (chrono::Utc::now().timestamp() - J2000_EPOCH as i64) as u32
}

/// Attempts to code a string as a fixed length string.
///
/// # Note
///
/// This does not add a null terminator!
pub(crate) fn encode_unterminated_fixed_string<const LEN: usize>(
    string: String,
) -> Result<[u8; LEN], EncodeError> {
    let mut encoded = [0u8; LEN];

    let string_bytes = string.into_bytes();
    if string_bytes.len() > encoded.len() {
        return Err(EncodeError::StringTooLong);
    }

    encoded[..string_bytes.len()].copy_from_slice(&string_bytes);

    Ok(encoded)
}

/// Attempts to encode a string as a fixed length string.
///
/// # Note
///
/// The output of this function will always be `LEN + 1` bytes on success.
pub(crate) fn encode_terminated_fixed_string<const LEN: usize>(string: String) -> Result<Vec<u8>, EncodeError> {
    let unterminated = encode_unterminated_fixed_string(string);

    unterminated.map(|bytes: [u8; LEN]| {
        let mut bytes = Vec::from(bytes);
        bytes.push(0);
        bytes
    })
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

        let size = self.payload.encode()?.len() as u16;
        encoded.extend(encode_var_u16(size)?);

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
pub struct HostBoundPacket<P, const ID: u8> {
    /// Host-bound Packet Header
    ///
    /// This must be `Self::HEADER` or `[0xAA, 0x55]`.
    header: [u8; 2],

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    payload: P,
}

impl<P, const ID: u8> HostBoundPacket<P, ID> {
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
