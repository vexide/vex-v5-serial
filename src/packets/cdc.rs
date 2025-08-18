use crate::{
    connection,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    varint::VarU16,
};

use super::{DEVICE_BOUND_HEADER, HOST_BOUND_HEADER};

/// Known CDC Command Identifiers
pub(crate) mod cmds {
    pub const ACK: u8 = 0x33;
    pub const QUERY_1: u8 = 0x21;
    pub const USER_CDC: u8 = 0x56;
    pub const CON_CDC: u8 = 0x58;
    pub const SYSTEM_VERSION: u8 = 0xA4;
    pub const EEPROM_ERASE: u8 = 0x31;
    pub const USER_ENTER: u8 = 0x60;
    pub const USER_CATALOG: u8 = 0x61;
    pub const FLASH_ERASE: u8 = 0x63;
    pub const FLASH_WRITE: u8 = 0x64;
    pub const FLASH_READ: u8 = 0x65;
    pub const USER_EXIT: u8 = 0x66;
    pub const USER_PLAY: u8 = 0x67;
    pub const USER_STOP: u8 = 0x68;
    pub const COMPONENT_GET: u8 = 0x69;
    pub const USER_SLOT_GET: u8 = 0x78;
    pub const USER_SLOT_SET: u8 = 0x79;
    pub const BRAIN_NAME_GET: u8 = 0x44;
}

/// CDC (Simple) Command Packet
///
/// Encodes a simple device-bound message over the protocol containing
/// a command identifier and a payload.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CdcCommandPacket<const CMD: u8, P: Encode> {
    payload: P,
}

impl<const CMD: u8, P: Encode> CdcCommandPacket<CMD, P> {
    /// Header used for device-bound VEX CDC packets.
    pub const HEADER: [u8; 4] = DEVICE_BOUND_HEADER;

    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self { payload }
    }
}

impl<const CMD: u8, P: Encode> Encode for CdcCommandPacket<CMD, P> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        // Push the header and CMD
        encoded.extend(Self::HEADER);
        encoded.push(CMD);

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

/// CDC (Simple) Command Reply Packet
///
/// Encodes a reply payload to a [`CdcCommandPacket`] for a given ID.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CdcReplyPacket<const CMD: u8, P: Decode> {
    /// Packet Payload Size
    pub payload_size: u16,

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    pub payload: P,
}

impl<const CMD: u8, P: Decode> CdcReplyPacket<CMD, P> {
    /// Header used for host-bound VEX CDC packets.
    pub const HEADER: [u8; 2] = HOST_BOUND_HEADER;
}

impl<const CMD: u8, P: Decode> Decode for CdcReplyPacket<CMD, P> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let header: [u8; 2] = Decode::decode(&mut data)?;
        if header != Self::HEADER {
            return Err(DecodeError::InvalidHeader);
        }
        
        let cmd = u8::decode(&mut data)?;
        if cmd != CMD {
            return Err(DecodeError::UnexpectedValue {
                value: cmd,
                expected: &[CMD],
            });
        }
        
        let payload_size = VarU16::decode(&mut data)?.into_inner();
        let payload = P::decode(data.take(payload_size as usize))?;

        Ok(Self {
            payload_size,
            payload,
        })
    }
}

impl<const CMD: u8, P: Decode> connection::CheckHeader for CdcReplyPacket<CMD, P> {
    fn has_valid_header(data: impl IntoIterator<Item = u8>) -> bool {
        let mut data = data.into_iter();
        if <[u8; 2] as Decode>::decode(&mut data)
            .map(|header| header != HOST_BOUND_HEADER)
            .unwrap_or(true)
        {
            return false;
        }

        if u8::decode(&mut data).map(|id| id != CMD).unwrap_or(true) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::CheckHeader;
    use crate::packets::file::FileDataReadReplyPacket;

    #[test]
    fn has_valid_header_success() {
        let data: &[u8] = &[
            0xaa, 0x55, 0x56, 0x7, 0x14, 0xd4, 0xff, 0xff, 0xff, 0xca, 0x3d,
        ];
        assert!(FileDataReadReplyPacket::has_valid_header(data.iter().cloned()));
    }
}
