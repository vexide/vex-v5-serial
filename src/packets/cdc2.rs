use super::{Decode, DecodeError, DeviceBoundPacket, Encode, EncodeError, HostBoundPacket};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
/// CDC2 Packet Acknowledgement Codes
pub enum Cdc2Ack {
    /// Acknowledges that a packet has been recieved successfully.
    Ack = 0x76,

    /// A general negative-acknowledgement (NACK) that is sometimes recieved.
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
            _ => Err(DecodeError::UnexpectedValue),
        }
    }
}

pub type Cdc2CommandPacket<const ID: u8, const EXT_ID: u8, P> =
    DeviceBoundPacket<Cdc2CommandPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandPayload<const ID: u8, P: Encode> {
    pub payload: P,
    pub crc: crc::Crc<u32>,
}
impl<const ID: u8, P: Encode> Encode for Cdc2CommandPayload<ID, P> {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        let payload_bytes = self.payload.encode()?;
        let hash = self.crc.checksum(&payload_bytes);

        encoded.extend(payload_bytes);
        encoded.extend_from_slice(&hash.to_be_bytes());

        Ok(encoded)
    }
}

pub type Cdc2ReplyPacket<const ID: u8, const EXT_ID: u8, P> =
    HostBoundPacket<Cdc2CommandReplyPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandReplyPayload<const ID: u8, P: Decode> {
    pub ack: Cdc2Ack,
    pub data: P,
    pub crc: u32,
}
impl<const ID: u8, P: Decode> Decode for Cdc2CommandReplyPayload<ID, P> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let ack = Cdc2Ack::decode(&mut data)?;
        let data_ = P::decode(&mut data)?;
        let crc = u32::decode(&mut data)?;

        Ok(Self { ack, data: data_, crc })
    }
}
