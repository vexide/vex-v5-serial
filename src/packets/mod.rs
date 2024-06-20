use std::fmt::Debug;
use crate::{
    encode::{Encode, EncodeError},
    decode::{Decode, DecodeError},
    varint::VarU16,
    crc::VEX_CRC16
};

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
        encoded.extend_from_slice(&self.payload.encode()?);
        if P::extended() {
            let hash = crc::Crc::<u16>::new(&VEX_CRC16).checksum(&encoded);
            encoded.extend(hash.to_be_bytes());
        }
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

pub(crate) fn decode_header(data: impl IntoIterator<Item = u8>) -> Result<[u8; 2], DecodeError> {
    let mut data = data.into_iter();
    let header = Decode::decode(&mut data)?;
    if header != [0xAA, 0x55] {
        return Err(DecodeError::InvalidHeader);
    }
    Ok(header)
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
    pub header: [u8; 2],

    /// Packet Payload Size
    pub payload_size: VarU16,
    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    pub payload: P,
}
impl<P: Decode + Debug, const ID: u8> Debug for HostBoundPacket<P, ID> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostBoundPacket")
            .field("header", &self.header)
            .field("payload_size", &self.payload_size)
            .field("payload", &self.payload)
            .finish()
    }
}

impl<P: Decode, const ID: u8> HostBoundPacket<P, ID> {
    /// Header byte sequence used for all host-bound packets.
    pub const HEADER: [u8; 2] = [0xAA, 0x55];
}
impl<P: Decode, const ID: u8> Decode for HostBoundPacket<P, ID> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let header = Decode::decode(&mut data)?;
        if header != Self::HEADER {
            return Err(DecodeError::InvalidHeader);
        }
        let id = u8::decode(&mut data)?;
        if id != ID {
            return Err(DecodeError::InvalidHeader);
        }
        let payload_size = VarU16::decode(&mut data)?;
        let payload = P::decode(data)?;

        Ok(Self {
            header,
            payload_size,
            payload,
        })
    }
}