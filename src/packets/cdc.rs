use std::fmt::Debug;

use crate::{
    connection, decode::{Decode, DecodeError, DecodeErrorKind}, encode::{Encode, EncodeError}, varint::VarU16
};

use super::{DEVICE_BOUND_HEADER, HOST_BOUND_HEADER};

/// CDC (Simple) Command Packet
///
/// Encodes a simple device-bound message over the protocol containing
/// an ID and a payload.
pub struct CdcCommandPacket<const ID: u8, P: Encode> {
    header: [u8; 4],
    payload: P,
}

impl<const ID: u8, P: Encode> CdcCommandPacket<ID, P> {
    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: DEVICE_BOUND_HEADER,
            payload,
        }
    }
}

impl<const ID: u8, P: Encode> Encode for CdcCommandPacket<ID, P> {
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

impl<const ID: u8, P: Encode + Clone> Clone for CdcCommandPacket<ID, P> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            payload: self.payload.clone(),
        }
    }
}

/// CDC (Simple) Command Reply Packet
///
/// Encodes a reply payload to a [`CdcCommandPacket`] for a given ID.
pub struct CdcReplyPacket<const ID: u8, P: Decode> {
    /// Host-bound Packet Header
    ///
    /// This must be `Self::HEADER` or `[0xAA, 0x55]`.
    pub header: [u8; 2],

    /// Packet Payload Size
    pub payload_size: u16,

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    pub payload: P,
}

impl<const ID: u8, P: Decode> Decode for CdcReplyPacket<ID, P> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let header = Decode::decode(&mut data)?;
        if header != HOST_BOUND_HEADER {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::InvalidHeader));
        }
        let id = u8::decode(&mut data)?;
        if id != ID {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::InvalidHeader));
        }
        let payload_size = VarU16::decode(&mut data)?.into_inner();
        let payload = P::decode(data.take(payload_size as usize))?;

        Ok(Self {
            header,
            payload_size,
            payload,
        })
    }
}

impl<const ID: u8, P: Decode> connection::CheckHeader for CdcReplyPacket<ID, P> {
    fn has_valid_header(data: impl IntoIterator<Item = u8>) -> bool {
        let mut data = data.into_iter();
        if <[u8; 2] as Decode>::decode(&mut data).map(|header| header != HOST_BOUND_HEADER).unwrap_or(true) {
            return false
        } 

        if u8::decode(&mut data).map(|id| id != ID).unwrap_or(true) {
            return false
        }

        true
    }
}

impl<const ID: u8, P: Decode + Debug> Debug for CdcReplyPacket<ID, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostBoundPacket")
            .field("header", &self.header)
            .field("payload_size", &self.payload_size)
            .field("payload", &self.payload)
            .finish()
    }
}

impl<const ID: u8, P: Decode + Clone> Clone for CdcReplyPacket<ID, P> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            payload_size: self.payload_size,
            payload: self.payload.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::file::ReadFileReplyPacket;
    use crate::connection::CheckHeader;

    #[test]
    fn has_valid_header_success() {
        let data: &[u8] = &[0xaa, 0x55, 0x56, 0x7, 0x14, 0xd4, 0xff, 0xff, 0xff, 0xca, 0x3d];
        assert!(ReadFileReplyPacket::has_valid_header(data.iter().cloned()));
    }
}
