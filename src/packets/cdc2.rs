use super::{DeviceBoundPacket, HostBoundPacket};

/// CDC2 Packet Acknowledgement Codes
pub enum Cdc2Ack {
    /// ACKnowledges that a packet has been recieved.
    Ack = 0x76,
    
    /// A general NACK that is sometimes recieved.
    Nack = 0xFF,
    
    /// Returned by the brain when a CDC2 packet's CRC Checksum does not validate.
    NackPacketCrc = 0xCE,
    
    /// Returned by the brain when a payload is too short.
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

pub type Cdc2CommandPacket<const ID: u8, const EXT_ID: u8, P> = DeviceBoundPacket<Cdc2CommandPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandPayload<const ID: u8, P> {
    pub payload: P,
    pub crc: crc::Algorithm<u32>,
}

pub type Cdc2ReplyPacket<const ID: u8, const EXT_ID: u8, P> = HostBoundPacket<Cdc2CommandReplyPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandReplyPayload<const ID: u8, P> {
    pub ack: Cdc2Ack,
    pub data: P,
    pub crc: crc::Algorithm<u32>,
}