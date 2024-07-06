use crate::{
    connection::ConnectionError,
    crc::VEX_CRC16,
    encode::{Encode, EncodeError},
    varint::VarU16,
};

use super::{DEVICE_BOUND_HEADER, HOST_BOUND_HEADER};
use crate::decode::{Decode, DecodeError};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
/// CDC2 Packet Acknowledgement Codes
pub enum Cdc2Ack {
    /// Acknowledges that a packet has been received successfully.
    Ack = 0x76,

    /// A general negative-acknowledgement (NACK) that is sometimes received.
    Nack = 0xFF,

    /// Returned by the brain when a CDC2 packet's CRC Checksum does not validate.
    NackPacketCrc = 0xCE,

    /// Returned by the brain when a packet's payload is of unexpected length (too short or too long).
    NackPacketLength = 0xD0,

    /// Returned by the brain when we attempt to transfer too much data.
    NackTransferSize = 0xD1,

    /// Returned by the brain when a program's CRC checksum fails.
    NackProgramCrc = 0xD2,

    /// Returned by the brain when there is an error with the program file.
    NackProgramFile = 0xD3,

    /// Returned by the brain when we fail to initialize a file transfer before beginning file operations.
    NackUninitializedTransfer = 0xD4,

    /// Returned by the brain when we initialize a file transfer incorrectly.
    NackInvalidInitialization = 0xD5,

    /// Returned by the brain when we fail to pad a transfer to a four byte boundary.
    NackAlignment = 0xD6,

    /// Returned by the brain when the addr on a file transfer does not match
    NackAddress = 0xD7,

    /// Returned by the brain when the download length on a file transfer does not match
    NackIncomplete = 0xD8,

    /// Returned by the brain when a file transfer attempts to access a directory that does not exist
    NackNoDirectory = 0xD9,

    /// Returned when the limit for user files has been reached
    NackMaxUserFiles = 0xDA,

    /// Returned when a file already exists and we did not specify overwrite when initializing the transfer
    NackFileAlreadyExists = 0xDB,

    /// Returned when the filesystem is full.
    NackFileStorageFull = 0xDC,

    /// Packet timed out.
    Timeout = 0x00,

    /// Internal Write Error.
    WriteError = 0x01,
}
impl Decode for Cdc2Ack {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let this = u8::decode(data)?;
        match this {
            0x76 => Ok(Self::Ack),
            0xFF => Ok(Self::Nack),
            0xCE => Ok(Self::NackPacketCrc),
            0xD0 => Ok(Self::NackPacketLength),
            0xD1 => Ok(Self::NackTransferSize),
            0xD2 => Ok(Self::NackProgramCrc),
            0xD3 => Ok(Self::NackProgramFile),
            0xD4 => Ok(Self::NackUninitializedTransfer),
            0xD5 => Ok(Self::NackInvalidInitialization),
            0xD6 => Ok(Self::NackAlignment),
            0xD7 => Ok(Self::NackAddress),
            0xD8 => Ok(Self::NackIncomplete),
            0xD9 => Ok(Self::NackNoDirectory),
            0xDA => Ok(Self::NackMaxUserFiles),
            0xDB => Ok(Self::NackFileAlreadyExists),
            0xDC => Ok(Self::NackFileStorageFull),
            0x00 => Ok(Self::Timeout),
            0x01 => Ok(Self::WriteError),
            v => Err(DecodeError::UnexpectedValue {
                value: v,
                expected: &[
                    0x76, 0xFF, 0xCE, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9,
                    0xDA, 0xDB, 0xDC, 0x00, 0x01,
                ],
            }),
        }
    }
}

pub struct Cdc2CommandPacket<const ID: u8, const EXT_ID: u8, P: Encode> {
    header: [u8; 4],
    payload: P,
    crc: crc::Crc<u16>,
}

impl<P: Encode, const ID: u8, const EXTENDED_ID: u8> Cdc2CommandPacket<ID, EXTENDED_ID, P> {
    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: DEVICE_BOUND_HEADER,
            payload,
            crc: VEX_CRC16,
        }
    }
}

impl<const ID: u8, const EXT_ID: u8, P: Encode> Encode for Cdc2CommandPacket<ID, EXT_ID, P> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend_from_slice(&self.header);

        // Push IDs
        encoded.push(ID);
        encoded.push(EXT_ID);

        // Push the payload size and encoded bytes
        let payload_bytes = self.payload.encode()?;
        let payload_size = VarU16::new(payload_bytes.len() as u16);
        encoded.extend(payload_size.encode()?);
        encoded.extend(payload_bytes);

        // The CRC32 checksum is of the whole encoded packet, meaning we need
        // to also include the header bytes.
        let checksum = self.crc.checksum(&encoded);

        encoded.extend(checksum.to_be_bytes());

        Ok(encoded)
    }
}

impl<const ID: u8, const EXT_ID: u8, P: Encode + Clone> Clone for Cdc2CommandPacket<ID, EXT_ID, P> {
    fn clone(&self) -> Self {
        Self {
            header: DEVICE_BOUND_HEADER,
            payload: self.payload.clone(),
            crc: self.crc.clone(),
        }
    }
}

pub struct Cdc2ReplyPacket<const ID: u8, const EXT_ID: u8, P: Decode> {
    pub header: [u8; 2],
    pub ack: Cdc2Ack,
    pub payload_size: VarU16,
    pub payload: P,
    pub crc: u16,
}

impl<const ID: u8, const EXT_ID: u8, P: Decode> Cdc2ReplyPacket<ID, EXT_ID, P> {
    pub fn try_into_inner(self) -> Result<P, ConnectionError> {
        if let Cdc2Ack::Ack = self.ack {
            Ok(self.payload)
        } else {
            Err(ConnectionError::Nack(self.ack))
        }
    }
}

impl<const ID: u8, const EXT_ID: u8, P: Decode> Decode for Cdc2ReplyPacket<ID, EXT_ID, P> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let header = Decode::decode(&mut data)?;
        if header != HOST_BOUND_HEADER {
            return Err(DecodeError::InvalidHeader);
        }

        let id = u8::decode(&mut data)?;
        if id != ID {
            return Err(DecodeError::InvalidHeader);
        }

        let payload_size = VarU16::decode(&mut data)?;

        let ext_id = u8::decode(&mut data)?;
        if ext_id != EXT_ID {
            return Err(DecodeError::InvalidHeader);
        }

        let ack = Cdc2Ack::decode(&mut data)?;
        let payload = P::decode(&mut data)?;
        let crc = u16::decode(&mut data)?;

        Ok(Self {
            header,
            ack,
            payload_size,
            payload,
            crc,
        })
    }
}

impl<const ID: u8, const EXT_ID: u8, P: Decode + Clone> Clone for Cdc2ReplyPacket<ID, EXT_ID, P> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            ack: self.ack,
            payload_size: self.payload_size,
            payload: self.payload.clone(),
            crc: self.crc,
        }
    }
}
