use thiserror::Error;

use crate::{
    crc::VEX_CRC16,
    decode::SizedDecode,
    encode::{Encode, EncodeError},
    varint::VarU16,
};

use super::{DEVICE_BOUND_HEADER, HOST_BOUND_HEADER};
use crate::decode::{Decode, DecodeError};

/// CDC2 Packet Acknowledgement Codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Error)]
pub enum Cdc2Ack {
    /// Acknowledges that a packet has been received successfully.
    #[error("Packet was recieved successfully. Wait, how'd this happen??")]
    Ack = 0x76,

    /// A general negative-acknowledgement (NACK) that is sometimes received.
    #[error("V5 device sent back a general negative-acknowledgement.")]
    Nack = 0xFF,

    /// Returned by the brain when a CDC2 packet's CRC Checksum does not validate.
    #[error("Packet CRC checksum did not validate. (NACK 0xCE)")]
    NackPacketCrc = 0xCE,

    /// Returned by the brain when a packet's payload is of unexpected length (too short or too long).
    #[error("Packet payload length was either too short or too long. (NACK 0xD0)")]
    NackPacketLength = 0xD0,

    /// Returned by the brain when we attempt to transfer too much data.
    #[error("Attempted to transfer too much data. (NACK 0xD1)")]
    NackTransferSize = 0xD1,

    /// Returned by the brain when a program's CRC checksum fails.
    #[error("Program CRC checksum did not validate. (NACK 0xD2)")]
    NackProgramCrc = 0xD2,

    /// Returned by the brain when there is an error with the program file.
    #[error("Invalid program file. (NACK 0xD3)")]
    NackProgramFile = 0xD3,

    /// Returned by the brain when we fail to initialize a file transfer before beginning file operations.
    #[error(
        "Attempted to perform a file transfer operation before one was initialized. (NACK 0xD4)"
    )]
    NackUninitializedTransfer = 0xD4,

    /// Returned by the brain when we initialize a file transfer incorrectly.
    #[error("File transfer was initialized incorrectly. (NACK 0xD5)")]
    NackInvalidInitialization = 0xD5,

    /// Returned by the brain when we fail to pad a transfer to a four byte boundary.
    #[error("File transfer was initialized incorrectly. (NACK 0xD6)")]
    NackAlignment = 0xD6,

    /// Returned by the brain when the addr on a file transfer does not match
    #[error("File transfer address did not match. (NACK 0xD7)")]
    NackAddress = 0xD7,

    /// Returned by the brain when the download length on a file transfer does not match
    #[error("File transfer download length did not match. (NACK 0xD8)")]
    NackIncomplete = 0xD8,

    /// Returned by the brain when a file transfer attempts to access a directory that does not exist
    #[error("Attempted to transfer file to a directory that does not exist. (NACK 0xD9)")]
    NackNoDirectory = 0xD9,

    /// Returned when the limit for user files has been reached
    #[error("Limit for user files has been reached. (NACK 0xDA)")]
    NackMaxUserFiles = 0xDA,

    /// Returned when a file already exists and we did not specify overwrite when initializing the transfer
    #[error("File already exists. (NACK 0xDB)")]
    NackFileAlreadyExists = 0xDB,

    /// Returned when the filesystem is full.
    #[error("Filesystem storage is full. (NACK 0xDC)")]
    NackFileStorageFull = 0xDC,

    /// Packet timed out.
    #[error("Packet timed out.")]
    Timeout = 0x00,

    /// Internal Write Error.
    #[error("Internal write error occurred.")]
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

pub struct Cdc2ReplyPacket<const ID: u8, const EXT_ID: u8, P: SizedDecode> {
    pub header: [u8; 2],
    pub ack: Cdc2Ack,
    pub payload_size: u16,
    pub payload: P,
    pub crc: u16,
}

impl<const ID: u8, const EXT_ID: u8, P: SizedDecode> Cdc2ReplyPacket<ID, EXT_ID, P> {
    pub fn try_into_inner(self) -> Result<P, Cdc2Ack> {
        if let Cdc2Ack::Ack = self.ack {
            Ok(self.payload)
        } else {
            Err(self.ack)
        }
    }
}

impl<const ID: u8, const EXT_ID: u8, P: SizedDecode> Decode for Cdc2ReplyPacket<ID, EXT_ID, P> {
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

        let payload_size = VarU16::decode(&mut data)?.into_inner();

        let ext_id = u8::decode(&mut data)?;
        if ext_id != EXT_ID {
            return Err(DecodeError::InvalidHeader);
        }

        let ack = Cdc2Ack::decode(&mut data)?;

        let payload = P::sized_decode(&mut data, payload_size.into())?;
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
