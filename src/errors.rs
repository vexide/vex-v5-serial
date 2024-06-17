use std::string::FromUtf8Error;

use thiserror::Error;

/// Represents an error in decoding a packet
#[derive(Error, Debug)]
pub enum DecodeError {
    /// Raised whenever there is an std::io::Error
    #[error("IO Error")]
    IoError(#[from] std::io::Error),
    /// Raised whenever there is an error decoding UTF8
    #[error("UTF8 decode error")]
    UTF8Error(#[from] FromUtf8Error),
    /// Raised when the timeout for recieving the packet header is reached
    #[error("timedout when waiting for header")]
    HeaderTimeout,
    /// Raised whenever we expected an extended packet but got garbage instead
    #[error("expected an extended packet")]
    ExpectedExtended,
    /// Raised whenever a CRC Checksum fails
    #[error("crc checksum failed")]
    CrcError,
    /// Raised whenever a packet length does not match the expected length
    #[error("packet length is incorrect")]
    PacketLengthError,
    /// Raised whenever an invalid ACK number is recieved
    #[error("invalid ack number")]
    InvalidAck,
    /// Raised whenever a NACK is recieved
    #[error("recieved a nack")]
    NACK(VexACKType),
    /// Raised whenever we recieve a response to a command that we did not expect a response to
    #[error("expected command _ recieved command _")]
    ExpectedCommand(u8, u8),
    /// Raised whenever a DeviceError is raised
    #[error("device error")]
    DeviceError(#[from] DeviceError),
    /// Raised whenever we encounter an invalid value
    #[error("invalid value")]
    InvalidValue(String),
}

/// Represents an error communicating with a device.
#[derive(Error, Debug)]
pub enum DeviceError {
    /// Raised whenever there is an std::io::Error
    #[error("IO Error")]
    IoError(#[from] std::io::Error),
    /// Raised whenever there is an error with a serial port
    #[error("Serialport Error")]
    SerialportError(#[from] tokio_serial::Error),
    /// Raised whenever a user attempts to write to the user port over wireless joystick access.
    #[error("The user port can not be currently written to over wireless control")]
    NoWriteOnWireless,
    /// Raised whenever a serial device is not a supported vex device
    #[error("The device is not a supported vex device")]
    InvalidDevice,
    /// Raised whenever we encounter an error with bluetooth.
    #[error("Bluetooth Error")]
    BluetoothError(#[from] bluest::Error),
    /// Raised when the user attempts to connect over bluetooth without a bluetooth adapter.
    #[error("No Bluetooth Adapter Found")]
    NoBluetoothAdapter,
    /// Raised whenever a user attempts to communicate with an unconnected device
    #[error("Not connected to the device")]
    NotConnected,
    /// Raised whenever a bluetooth device returns an invalid magic number
    #[error("Invalid Magic Number")]
    InvalidMagic,
}

/// A V5 device can respond with various different acknowledgements.
/// Some, known as NACKs, are errors that the device cannot handle.
/// This list contains all known NACKs as well as ACK.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VexACKType {
    /// ACKnowledges that a packet has been recieved.
    ACK = 0x76,
    /// Returned by the brain when a CRC Checksum does not validate
    NACKCrcError = 0xCE,
    /// Returned by the brain when a payload is too short
    NACKPayloadShort = 0xD0,
    /// Returned by the brain when we attempt to transfer too much data
    NACKTransferSizeTooLarge = 0xD1,
    /// Returned by the brain when a program's CRC check fails
    NACKProgramCrcFailed = 0xD2,
    /// Returned by the brain when there is an error with the program file
    NACKProgramFileError = 0xD3,
    /// Returned by the brain when we fail to initialize a file transfer before beginning file operations
    NACKUninitializedTransfer = 0xD4,
    /// Returned by the brain when we initialize a file transfer incorrectly.
    NACKInitializationInvalid = 0xD5,
    /// Returned by the brain when we fail to pad a transfer to a four byte boundary.
    NACKLengthNotPaddedTo4 = 0xD6,
    /// Returned by the brain when the addr on a file transfer does not match
    NACKAddressNoMatch = 0xD7,
    /// Returned by the brain when the download length on a file transfer does not match
    NACKDownloadLengthNoMatch = 0xD8,
    /// Returned by the brain when a file transfer attempts to access a directory that does not exist
    NACKDirectoryNoExist = 0xD9,
    /// Returned when there is not enough room to upload a file
    NACKNoFileRoom = 0xDA,
    /// Returned when a file already exists and we did not specify overwrite when initializing the transfer
    NACKFileAlreadyExists = 0xDB,
    /// A general NACK that is sometimes recieved.
    NACKGeneral = 0xFF,
}

impl VexACKType {
    /// Converts a [u8] to a variant of [VexACKType] based on the value of the ACK.
    pub fn from_u8(v: u8) -> Result<Self, DecodeError> {
        match v {
            0x76 => Ok(Self::ACK),
            0xCE => Ok(Self::NACKCrcError),
            0xD0 => Ok(Self::NACKPayloadShort),
            0xD1 => Ok(Self::NACKTransferSizeTooLarge),
            0xD2 => Ok(Self::NACKProgramCrcFailed),
            0xD3 => Ok(Self::NACKProgramFileError),
            0xD4 => Ok(Self::NACKUninitializedTransfer),
            0xD5 => Ok(Self::NACKInitializationInvalid),
            0xD6 => Ok(Self::NACKLengthNotPaddedTo4),
            0xD7 => Ok(Self::NACKAddressNoMatch),
            0xD8 => Ok(Self::NACKDownloadLengthNoMatch),
            0xD9 => Ok(Self::NACKDirectoryNoExist),
            0xDA => Ok(Self::NACKNoFileRoom),
            0xDB => Ok(Self::NACKFileAlreadyExists),
            _ => Err(DecodeError::InvalidAck),
        }
    }
}
