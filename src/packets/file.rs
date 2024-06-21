//! Filesystem Access

use std::vec;

use super::{
    cdc2::{Cdc2Ack, Cdc2CommandPacket, Cdc2ReplyPacket},
    HostBoundPacket,
};
use crate::{
    array::Array,
    choice::{Choice, PrefferedChoice},
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    string::FixedLengthString,
    version::Version,
};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileInitAction {
    Write = 1,
    Read = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileInitOption {
    None = 0,
    Overwrite = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileDownloadTarget {
    Ddr = 0,
    Qspi = 1,
    Cbuf = 2,
    Vbuf = 3,
    Ddrc = 4,
    Ddre = 5,
    Flash = 6,
    Radio = 7,
    A1 = 13,
    B1 = 14,
    B2 = 15,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileVendor {
    User = 1,
    Sys = 15,
    Dev1 = 16,
    Dev2 = 24,
    Dev3 = 32,
    Dev4 = 40,
    Dev5 = 48,
    Dev6 = 56,
    VexVm = 64,
    Vex = 240,
    Undefined = 241,
}
impl Decode for FileVendor {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let this = u8::decode(data)?;
        match this {
            1 => Ok(Self::User),
            15 => Ok(Self::Sys),
            16 => Ok(Self::Dev1),
            24 => Ok(Self::Dev2),
            32 => Ok(Self::Dev3),
            40 => Ok(Self::Dev4),
            48 => Ok(Self::Dev5),
            56 => Ok(Self::Dev6),
            64 => Ok(Self::VexVm),
            240 => Ok(Self::Vex),
            241 => Ok(Self::Undefined),
            _ => Err(DecodeError::UnexpectedValue),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileLoadAction {
    Run = 0,
    Stop = 1,
}

/// Start uploading or downloading file from the device
pub type InitFileTransferPacket = Cdc2CommandPacket<0x56, 0x11, InitFileTransferPayload>;
pub type InitFileTransferReplyPacket = Cdc2ReplyPacket<0x56, 0x11, InitFileTransferReplyPayload>;

pub struct InitFileTransferPayload {
    pub operation: FileInitAction,
    pub target: FileDownloadTarget,
    pub vendor: FileVendor,
    pub options: FileInitOption,
    pub write_file_size: u32,
    pub load_address: u32,
    pub write_file_crc: u32,
    pub file_extension: FixedLengthString<3>,
    pub timestamp: i32,
    pub version: Version,
    pub file_name: FixedLengthString<23>,
}

impl Encode for InitFileTransferPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![
            self.operation as _,
            self.target as _,
            self.vendor as _,
            self.options as _,
        ];
        encoded.extend(self.write_file_size.to_le_bytes());
        encoded.extend(self.load_address.to_le_bytes());
        encoded.extend(self.write_file_crc.to_be_bytes());
        encoded.extend(self.file_extension.encode()?);
        encoded.extend(self.timestamp.to_le_bytes());
        encoded.extend(self.version.encode()?);
        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

pub struct InitFileTransferReplyPayload {
    /// The amount of receive data (in bytes) that can be sent in every packet.
    pub window_size: u16,

    /// In read operation, the device returns the target file size (in bytes).
    ///
    /// In write operation, the device returns the value 3145728.
    pub file_size: u32,

    /// In read operation, the device returns the CRC value of the target file.
    ///
    /// In write operation, the device returns the same CRC value as the previous packets.
    pub file_crc: u32,
}

impl Decode for InitFileTransferReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let window_size = u16::decode(&mut data)?;
        let file_size = u32::decode(&mut data)?;
        // Convert from big endian
        let file_crc = u32::decode(&mut data)?.swap_bytes();
        Ok(Self {
            window_size,
            file_size,
            file_crc,
        })
    }
}

/// Finish uploading or downloading file from the device
pub type ExitFileTransferPacket = Cdc2CommandPacket<0x56, 0x12, FileExitAtion>;
pub type ExitFileTransferReplyPacket = Cdc2ReplyPacket<0x56, 0x12, ()>;

/// The action to run when a file transfer is completed.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileExitAtion {
    DoNothing = 0,
    RunProgram = 1,
    ShowRunScreen = 2,
    Halt = 3,
}
impl Encode for FileExitAtion {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![*self as _])
    }
}
/// Write to the brain
pub type WriteFilePacket = Cdc2CommandPacket<0x56, 0x13, WriteFilePayload>;
pub type WriteFileReplyPacket = Cdc2ReplyPacket<0x56, 0x13, ()>;

pub struct WriteFilePayload {
    /// Memory address to write to.
    pub address: i32,

    /// A sequence of bytes to write. Must be 4-byte aligned.
    pub chunk_data: Vec<u8>,
}
impl Encode for WriteFilePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend(self.address.to_le_bytes());
        encoded.extend(&self.chunk_data);

        Ok(encoded)
    }
}

/// Read from the brain
pub type ReadFilePacket = Cdc2CommandPacket<0x56, 0x14, ReadFilePayload>;
/// Returns the file content. This packet doesn't have an ack if the data is available.
pub type ReadFileReplyPacket = HostBoundPacket<ReadFileReplyPayload, 0x56>;

pub struct ReadFilePayload {
    /// Memory address to read from.
    pub address: u32,

    /// Number of bytes to read (4-byte aligned).
    pub size: u16,
}
impl Encode for ReadFilePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.address.to_le_bytes());
        encoded.extend(self.size.to_le_bytes());
        Ok(encoded)
    }
}

pub enum ReadFileReplyContents {
    Failure {
        nack: Cdc2Ack,
    },
    Success {
        /// Memory address to read from.
        address: u32,
        data: Array<u8>,
    },
}
impl Decode for ReadFileReplyContents {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        struct Success {
            address: u32,
            data: Array<u8>,
        }
        impl Decode for Success {
            fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
                let mut data = data.into_iter();
                let address = u32::decode(&mut data)?;

                // This is a cursed way to get the number of bytes in chunk_data.
                let data_vec = data.collect::<Vec<_>>();
                // The last two bytes are the CRC checksum.
                let num_bytes = data_vec.len() - 2;
                let data = data_vec.into_iter();

                let chunk_data = Array::decode_with_len(data, num_bytes)?;
                Ok(Self {
                    address,
                    data: chunk_data,
                })
            }
        }
        struct Failure {
            nack: Cdc2Ack,
        }
        impl Decode for Failure {
            fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
                let nack = Cdc2Ack::decode(data)?;
                Ok(Self { nack })
            }
        }

        let result = Choice::<Success, Failure>::decode(data)?.prefer_left();
        Ok(match result {
            PrefferedChoice::Left(success) => Self::Success {
                address: success.address,
                data: success.data,
            },
            PrefferedChoice::Right(failure) => Self::Failure { nack: failure.nack },
        })
    }
}

pub struct ReadFileReplyPayload {
    pub contents: ReadFileReplyContents,
    pub crc: u16,
}
impl Decode for ReadFileReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut data = data.into_iter();
        let id = u8::decode(&mut data)?;
        if id != 0x14 {
            return Err(DecodeError::UnexpectedValue);
        }
        let contents = ReadFileReplyContents::decode(&mut data)?;
        // the checksum is in big endian.
        let crc = u16::decode(&mut data)?.swap_bytes();

        Ok(Self { contents, crc })
    }
}
impl ReadFileReplyPayload {
    pub fn unwrap(self) -> Result<(u32, Array<u8>), Cdc2Ack> {
        match self.contents {
            ReadFileReplyContents::Success { address, data } => Ok((address, data)),
            ReadFileReplyContents::Failure { nack } => Err(nack),
        }
    }
}

/// File linking means allowing one file to be loaded after another file first (its parent).
///
/// This is used in PROS for the hot/cold linking.
pub type LinkFilePacket = Cdc2CommandPacket<0x56, 0x15, LinkFilePayload>;
pub type LinkFileReplyPacket = Cdc2ReplyPacket<0x56, 0x15, ()>;

pub struct LinkFilePayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub required_file: FixedLengthString<23>,
}
impl Encode for LinkFilePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option as _];
        let string = self.required_file.encode()?;
        encoded.extend(string);

        Ok(encoded)
    }
}

pub type GetDirectoryFileCountPacket = Cdc2CommandPacket<0x56, 0x16, GetDirectoryFileCountPayload>;
pub type GetDirectoryFileCountReplyPacket = Cdc2ReplyPacket<0x56, 0x16, u16>;

pub struct GetDirectoryFileCountPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}
impl Encode for GetDirectoryFileCountPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.vendor as _, self.option])
    }
}

pub type GetDirectoryEntryPacket = Cdc2CommandPacket<0x56, 0x17, GetDirectoryEntryPayload>;
pub type GetDirectoryEntryReplyPacket =
    Cdc2ReplyPacket<0x56, 0x17, Option<GetDirectoryEntryReplyPayload>>;

pub struct GetDirectoryEntryPayload {
    pub file_index: u8,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}
impl Encode for GetDirectoryEntryPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.file_index, self.option])
    }
}

pub struct GetDirectoryEntryReplyPayload {
    pub file_index: u8,
    pub size: u32,

    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc: u32,
    pub file_type: FixedLengthString<3>,

    /// The unix epoch timestamp minus [`J2000_EPOCH`].
    pub timestamp: i32,
    pub version: Version,
    pub file_name: FixedLengthString<23>,
}

impl Decode for GetDirectoryEntryReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let file_index = u8::decode(&mut data)?;
        let size = u32::decode(&mut data)?;
        let load_address = u32::decode(&mut data)?;
        let crc = u32::decode(&mut data)?;
        let file_type = Decode::decode(&mut data)?;
        let timestamp = i32::decode(&mut data)?;
        let version = Version::decode(&mut data)?;
        let file_name = Decode::decode(&mut data)?;

        Ok(Self {
            file_index,
            size,
            load_address,
            crc,
            file_type,
            timestamp,
            version,
            file_name,
        })
    }
}

/// Run a binrary file on the brain or stop the program running on the brain.
pub type LoadFileActionPacket = Cdc2CommandPacket<0x56, 0x18, LoadFileActionPayload>;
pub type LoadFileActionReplyPacket = Cdc2ReplyPacket<0x56, 0x18, ()>;

pub struct LoadFileActionPayload {
    pub vendor: FileVendor,
    pub action: FileInitAction,
    pub file_name: FixedLengthString<23>,
}
impl Encode for LoadFileActionPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.action as _];
        let string = self.file_name.encode()?;
        encoded.extend(string);

        Ok(encoded)
    }
}
pub type GetFileMetadataPacket = Cdc2CommandPacket<0x56, 0x19, GetFileMetadataPayload>;
pub type GetFileMetadataReplyPacket =
    Cdc2ReplyPacket<0x56, 0x19, Option<GetFileMetadataReplyPayload>>;

pub struct GetFileMetadataPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub file_name: FixedLengthString<23>,
}
impl Encode for GetFileMetadataPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];
        let string = self.file_name.encode()?;
        encoded.extend(string);

        Ok(encoded)
    }
}

pub struct GetFileMetadataReplyPayload {
    /// RESEARCH NEEDED: Unknown what this is if there is no link to the file.
    pub linked_vendor: FileVendor,
    pub size: u32,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc32: u32,
    pub file_type: FixedLengthString<3>,
    /// The unix epoch timestamp minus [`J2000_EPOCH`].
    pub timestamp: i32,
    pub version: Version,
}
impl Decode for GetFileMetadataReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let linked_vendor = FileVendor::decode(&mut data)?;
        let size = u32::decode(&mut data)?;
        let load_address = u32::decode(&mut data)?;
        let crc32 = u32::decode(&mut data)?;
        let file_type = Decode::decode(&mut data)?;
        let timestamp = i32::decode(&mut data)?;
        let version = Version::decode(&mut data)?;

        Ok(Self {
            linked_vendor,
            size,
            load_address,
            crc32,
            file_type,
            timestamp,
            version,
        })
    }
}

pub type SetFileMetadataPacket = Cdc2CommandPacket<0x56, 0x1a, SetFileMetadataPayload>;
pub type SetFileMetadataReplyPacket = Cdc2ReplyPacket<0x56, 0x1a, ()>;

pub struct SetFileMetadataPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub file_type: FixedLengthString<3>,
    pub timestamp: i32,
    pub version: Version,
    pub file_name: FixedLengthString<23>,
}
impl Encode for SetFileMetadataPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];
        encoded.extend(self.load_address.to_le_bytes());
        encoded.extend(self.file_type.encode()?);
        encoded.extend(self.timestamp.to_le_bytes());
        encoded.extend(self.version.encode()?);
        encoded.extend(self.file_name.encode()?);
        Ok(encoded)
    }
}

pub type EraseFilePacket = Cdc2CommandPacket<0x56, 0x1b, EraseFilePayload>;
pub type EraseFileReplyPacket = Cdc2ReplyPacket<0x56, 0x1b, ()>;

pub struct EraseFilePayload {
    pub vendor: FileVendor,
    /// 128 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub file_name: FixedLengthString<23>,
}
impl Encode for EraseFilePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];
        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

pub type FileClearUpPacket = Cdc2CommandPacket<0x56, 0x1e, FileClearUpPayload>;
pub type FileClearUpReplyPacket = Cdc2CommandPacket<0x56, 0x1e, FileClearUpResult>;

pub struct FileClearUpPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}
impl Encode for FileClearUpPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.vendor as _, self.option])
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
/// (RESEARCH NEEDED)
pub enum FileClearUpResult {
    /// No file deleted
    None = 0,

    /// Deleted all files
    AllFiles = 1,

    /// Deleted all files with linked files
    LinkedFiles = 2,

    /// Deleted all files for the first time after restart
    AllFilesAfterRestart = 3,

    /// Deleted all files with linked files for the first time after restart.
    LinkedFilesAfterRestart = 4,
}
impl Decode for FileClearUpResult {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let this = u8::decode(data)?;
        match this {
            0 => Ok(Self::None),
            1 => Ok(Self::AllFiles),
            2 => Ok(Self::LinkedFiles),
            3 => Ok(Self::AllFilesAfterRestart),
            4 => Ok(Self::LinkedFilesAfterRestart),
            _ => Err(DecodeError::UnexpectedValue),
        }
    }
}

/// Same as "File Clear Up", but takes longer
pub type FileFormatPacket = Cdc2CommandPacket<0x56, 0x1f, FileFormatConfirmation>;
pub type FileFormatReplyPacket = Cdc2CommandPacket<0x56, 0x1f, ()>;

pub struct FileFormatConfirmation {
    /// Must be [0x44, 0x43, 0x42, 0x41].
    pub confirmation_code: [u8; 4],
}
impl Encode for FileFormatConfirmation {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.confirmation_code.to_vec())
    }
}
