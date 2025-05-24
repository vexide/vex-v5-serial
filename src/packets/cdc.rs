use std::fmt::Debug;

use crate::{
    connection,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    varint::VarU16,
};

use super::{DEVICE_BOUND_HEADER, HOST_BOUND_HEADER};

/// CDC (Simple) Command Packet
///
/// Encodes a simple device-bound message over the protocol containing
/// an ID and a payload.
#[derive(Debug, Eq, PartialEq)]
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

impl<const CMD: u8, P: Encode + Clone> Clone for CdcCommandPacket<CMD, P> {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
        }
    }
}

/// CDC (Simple) Command Reply Packet
///
/// Encodes a reply payload to a [`CdcCommandPacket`] for a given ID.
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

impl<const CMD: u8, P: Decode + Debug> Debug for CdcReplyPacket<CMD, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!(
            "CdcReplyPacket<{CMD}, {}>",
            std::any::type_name::<P>()
        ))
        .field("payload_size", &self.payload_size)
        .field("payload", &self.payload)
        .finish()
    }
}

impl<const CMD: u8, P: Decode + Clone> Clone for CdcReplyPacket<CMD, P> {
    fn clone(&self) -> Self {
        Self {
            payload_size: self.payload_size,
            payload: self.payload.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::connection::CheckHeader;
    use crate::packets::file::ReadFileReplyPacket;

    #[test]
    fn has_valid_header_success() {
        let data: &[u8] = &[
            0xaa, 0x55, 0x56, 0x7, 0x14, 0xd4, 0xff, 0xff, 0xff, 0xca, 0x3d,
        ];
        assert!(ReadFileReplyPacket::has_valid_header(data.iter().cloned()));
    }
}
