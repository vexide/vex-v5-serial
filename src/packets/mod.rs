use crate::{
    crc::VEX_CRC16,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    varint::VarU16,
};
use std::fmt::Debug;

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

/// Device-bound CDC Packet
///
/// This structure encodes a data payload that is intended to be sent from a host
/// machine to a V5 device over the serial protocol.
pub struct DeviceBoundCdcPacket<const ID: u8, P: Encode> {
    header: [u8; 4],
    payload: P,
}
impl<P: Encode, const ID: u8> Encode for DeviceBoundCdcPacket<ID, P> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        // Push the header and ID
        encoded.extend_from_slice(&self.header);
        encoded.push(ID);

        let payload_bytes = self.payload.encode()?;

        // We only encode the payload size if there is a payload
        if !payload_bytes.is_empty() {
            let size = VarU16::new(payload_bytes.len() as _);
            encoded.extend(size.encode()?);
            encoded.extend(payload_bytes);
        }

        Ok(encoded)
    }
}
impl<P: Encode, const ID: u8> DeviceBoundCdcPacket<ID, P> {
    pub const HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

    pub fn new(payload: P) -> Self {
        Self {
            header: Self::HEADER,
            payload,
        }
    }
}
impl<P: Encode + Clone, const ID: u8> Clone for DeviceBoundCdcPacket<ID, P> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            payload: self.payload.clone(),
        }
    }
}

/// Device-bound CDC2 Packet
///
/// This structure encodes a data payload, ID, EXTENDED_ID, and CRC checksum that is intended to be sent from
/// a host machine to a V5 device over the serial protocol.
pub struct DeviceBoundCdc2Packet<const ID: u8, const EXTENDED_ID: u8, P: Encode> {
    /// Device-bound Packet Header
    ///
    /// This must be `Self::HEADER` or `[0xC9, 0x36, 0xB8, 0x47]`.
    header: [u8; 4],

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the device.
    payload: P,

    /// Packet CRC generator
    crc: crc::Crc<u16>,
}
impl<P: Encode, const ID: u8, const EXTENDED_ID: u8> Encode
    for DeviceBoundCdc2Packet<ID, EXTENDED_ID, P>
{
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        // Push the header
        encoded.extend_from_slice(&self.header);

        // Push the ID and extended ID
        encoded.push(ID);
        encoded.push(EXTENDED_ID);

        // Push the payload and payload size
        let payload_bytes = self.payload.encode()?;
        let size = VarU16::new(payload_bytes.len() as _);
        encoded.extend(size.encode()?);
        encoded.extend(payload_bytes);

        // Push the CRC checksum of the entire packet
        let hash = self.crc.checksum(&encoded);
        encoded.extend(hash.to_be_bytes());

        Ok(encoded)
    }
}
impl<P: Encode + Clone, const ID: u8, const EXTENDED_ID: u8> Clone
    for DeviceBoundCdc2Packet<ID, EXTENDED_ID, P>
{
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            payload: self.payload.clone(),
            crc: self.crc.clone(),
        }
    }
}

impl<P: Encode, const ID: u8, const EXTENDED_ID: u8> DeviceBoundCdc2Packet<ID, EXTENDED_ID, P> {
    /// Header byte sequence used for all device-bound packets.
    pub const HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: Self::HEADER,
            payload,
            crc: VEX_CRC16,
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
impl<P: Decode + Clone, const ID: u8> Clone for HostBoundPacket<P, ID> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            payload_size: self.payload_size,
            payload: self.payload.clone(),
        }
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
