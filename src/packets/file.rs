//! Filesystem Access

use std::{str, vec};

use super::{
    cdc::cmds::USER_CDC,
    cdc::CdcReplyPacket,
    cdc2::{
        ecmds::{
            FILE_CLEANUP, FILE_CTRL, FILE_DIR, FILE_DIR_ENTRY, FILE_ERASE, FILE_EXIT, FILE_FORMAT,
            FILE_GET_INFO, FILE_INIT, FILE_LINK, FILE_LOAD, FILE_READ, FILE_SET_INFO, FILE_WRITE,
        },
        Cdc2Ack, Cdc2CommandPacket, Cdc2ReplyPacket,
    },
};
use crate::{
    choice::{Choice, PrefferedChoice},
    decode::{Decode, DecodeError, SizedDecode},
    encode::{Encode, EncodeError},
    string::FixedString,
    version::Version,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum FileTransferOperation {
    /// Write (upload) a file.
    Write = 1,

    /// Read (download) a file.
    Read = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileInitOption {
    None = 0,
    Overwrite = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileTransferTarget {
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
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
            v => Err(DecodeError::UnexpectedValue {
                value: v,
                expected: &[
                    0x01, 0x0F, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x40, 0xF0, 0xF1,
                ],
            }),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileLoadAction {
    Run = 0,
    Stop = 128,
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ExtensionType {
    /// Regular unencrypted file.
    #[default]
    Binary = 0x0,

    /// Unknown use
    Vm = 0x61,

    /// File's contents is encrypted.
    EncryptedBinary = 0x73,
}

impl Decode for ExtensionType {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        Ok(match u8::decode(data)? {
            0x0 => Self::Binary,
            0x61 => Self::Vm,
            0x73 => Self::EncryptedBinary,
            unknown => {
                return Err(DecodeError::UnexpectedValue {
                    value: unknown,
                    expected: &[0x0],
                })
            }
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadata {
    pub extension: FixedString<3>,
    pub extension_type: ExtensionType,
    pub timestamp: i32,
    pub version: Version,
}

impl Encode for FileMetadata {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut data = vec![0; 3];
        // extension is not null terminated and is fixed length
        data[..self.extension.as_ref().len()].copy_from_slice(self.extension.as_ref().as_bytes());
        data.push(self.extension_type as _);
        data.extend(self.timestamp.to_le_bytes());
        data.extend(self.version.encode()?);

        Ok(data)
    }
}

impl Decode for FileMetadata {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut data = data.into_iter();

        Ok(Self {
            // SAFETY: length is guaranteed to be less than 4.
            extension: unsafe {
                FixedString::new_unchecked(
                    str::from_utf8(&<[u8; 3]>::decode(&mut data)?)?.to_string(),
                )
            },
            extension_type: Decode::decode(&mut data).unwrap(),
            timestamp: i32::decode(&mut data)?,
            version: Version::decode(&mut data)?,
        })
    }
}

/// Start uploading or downloading file from the device
pub type FileTransferInitializePacket =
    Cdc2CommandPacket<USER_CDC, FILE_INIT, FileTransferInitializePayload>;
pub type FileTransferInitializeReplyPacket =
    Cdc2ReplyPacket<USER_CDC, FILE_INIT, FileTransferInitializeReplyPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileTransferInitializePayload {
    pub operation: FileTransferOperation,
    pub target: FileTransferTarget,
    pub vendor: FileVendor,
    pub options: FileInitOption,
    pub file_size: u32,
    pub load_address: u32,
    pub write_file_crc: u32,
    pub metadata: FileMetadata,
    pub file_name: FixedString<23>,
}

impl Encode for FileTransferInitializePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![
            self.operation as _,
            self.target as _,
            self.vendor as _,
            self.options as _,
        ];
        encoded.extend(self.file_size.to_le_bytes());
        encoded.extend(self.load_address.to_le_bytes());
        encoded.extend(self.write_file_crc.to_le_bytes());
        encoded.extend(self.metadata.encode()?);
        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileTransferInitializeReplyPayload {
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

impl Decode for FileTransferInitializeReplyPayload {
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
pub type FileTransferExitPacket = Cdc2CommandPacket<USER_CDC, FILE_EXIT, FileExitAction>;
pub type FileTransferExitReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_EXIT, ()>;

/// The action to run when a file transfer is completed.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileExitAction {
    DoNothing = 0,
    RunProgram = 1,
    Halt = 2,
    ShowRunScreen = 3,
}
impl Encode for FileExitAction {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![*self as _])
    }
}
/// Write to the brain
pub type FileDataWritePacket = Cdc2CommandPacket<USER_CDC, FILE_WRITE, FileDataWritePayload>;
pub type FileDataWriteReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_WRITE, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileDataWritePayload {
    /// Memory address to write to.
    pub address: i32,

    /// A sequence of bytes to write. Must be 4-byte aligned.
    pub chunk_data: Vec<u8>,
}
impl Encode for FileDataWritePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend(self.address.to_le_bytes());
        encoded.extend(&self.chunk_data);

        Ok(encoded)
    }
}

/// Read from the brain
pub type FileDataReadPacket = Cdc2CommandPacket<USER_CDC, FILE_READ, FileDataReadPayload>;
/// Returns the file content. This packet doesn't have an ack if the data is available.
pub type FileDataReadReplyPacket = CdcReplyPacket<USER_CDC, FileDataReadReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileDataReadPayload {
    /// Memory address to read from.
    pub address: u32,

    /// Number of bytes to read (4-byte aligned).
    pub size: u16,
}
impl Encode for FileDataReadPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.address.to_le_bytes());
        encoded.extend(self.size.to_le_bytes());
        Ok(encoded)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FileDataReadReplyContents {
    Failure {
        nack: Cdc2Ack,
        crc: u16,
    },
    Success {
        /// Memory address to read from.
        address: u32,
        data: Vec<u8>,
        crc: u16,
    },
}
impl Decode for FileDataReadReplyContents {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        struct Success {
            address: u32,
            data: Vec<u8>,
            crc: u16,
        }
        impl Decode for Success {
            fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
                let mut data = data.into_iter();
                let address = u32::decode(&mut data)?;

                // This is a cursed way to get the number of bytes in chunk_data.
                let data_vec = data.collect::<Vec<_>>();
                // The last two bytes are the CRC checksum.
                let num_bytes = data_vec.len() - 2;
                let mut data = data_vec.into_iter();

                let chunk_data = Vec::sized_decode(&mut data, num_bytes as _)?;
                let crc = u16::decode(&mut data)?.swap_bytes();
                Ok(Self {
                    address,
                    data: chunk_data,
                    crc,
                })
            }
        }
        struct Failure {
            nack: Cdc2Ack,
            crc: u16,
        }
        impl Decode for Failure {
            fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
                let mut data = data.into_iter();
                let nack = Cdc2Ack::decode(&mut data)?;
                let crc = u16::decode(&mut data)?.swap_bytes();
                Ok(Self { nack, crc })
            }
        }

        let result = Choice::<Success, Failure>::decode(data)?.prefer_left();
        Ok(match result {
            PrefferedChoice::Left(success) => Self::Success {
                address: success.address,
                data: success.data,
                crc: success.crc,
            },
            PrefferedChoice::Right(failure) => Self::Failure {
                nack: failure.nack,
                crc: failure.crc,
            },
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileDataReadReplyPayload {
    pub contents: FileDataReadReplyContents,
}
impl Decode for FileDataReadReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut data = data.into_iter();
        let id = u8::decode(&mut data)?;
        if id != 0x14 {
            return Err(DecodeError::UnexpectedValue {
                value: id,
                expected: &[0x14],
            });
        }
        let contents = FileDataReadReplyContents::decode(&mut data)?;
        Ok(Self { contents })
    }
}
impl FileDataReadReplyPayload {
    pub fn unwrap(self) -> Result<(u32, Vec<u8>), Cdc2Ack> {
        match self.contents {
            FileDataReadReplyContents::Success {
                address,
                data,
                crc: _,
            } => Ok((address, data)),
            FileDataReadReplyContents::Failure { nack, crc: _ } => Err(nack),
        }
    }
}

/// File linking means allowing one file to be loaded after another file first (its parent).
///
/// This is used in PROS for the hot/cold linking.
pub type FileLinkPacket = Cdc2CommandPacket<USER_CDC, FILE_LINK, FileLinkPayload>;
pub type FileLinkReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_LINK, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileLinkPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub required_file: FixedString<23>,
}
impl Encode for FileLinkPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option as _];
        let string = self.required_file.encode()?;
        encoded.extend(string);

        Ok(encoded)
    }
}

pub type DirectoryFileCountPacket =
    Cdc2CommandPacket<USER_CDC, FILE_DIR, DirectoryFileCountPayload>;
pub type DirectoryFileCountReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_DIR, u16>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DirectoryFileCountPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}
impl Encode for DirectoryFileCountPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.vendor as _, self.option])
    }
}

pub type DirectoryEntryPacket =
    Cdc2CommandPacket<USER_CDC, FILE_DIR_ENTRY, DirectoryEntryPayload>;
pub type DirectoryEntryReplyPacket =
    Cdc2ReplyPacket<USER_CDC, FILE_DIR_ENTRY, Option<DirectoryEntryReplyPayload>>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DirectoryEntryPayload {
    pub file_index: u8,
    /// 0 = default. (RESEARCH NEEDED)
    pub unknown: u8,
}
impl Encode for DirectoryEntryPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.file_index, self.unknown])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntryReplyPayload {
    pub file_index: u8,
    pub size: u32,

    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc: u32,

    pub metadata: Option<FileMetadata>,
    pub file_name: String,
}

impl Decode for DirectoryEntryReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let file_index = u8::decode(&mut data)?;
        let size = u32::decode(&mut data)?;
        let load_address = u32::decode(&mut data)?;
        let crc = u32::decode(&mut data)?;

        let mut data = data.peekable();

        let metadata = if data.peek() == Some(&255) {
            let _ = <[u8; 12]>::decode(&mut data);
            None
        } else {
            Some(FileMetadata::decode(&mut data)?)
        };

        let file_name = FixedString::<23>::decode(&mut data)?.into_inner();

        Ok(Self {
            file_index,
            size,
            load_address,
            crc,
            metadata,
            file_name,
        })
    }
}

/// Run a binrary file on the brain or stop the program running on the brain.
pub type FileLoadActionPacket = Cdc2CommandPacket<USER_CDC, FILE_LOAD, FileLoadActionPayload>;
pub type FileLoadActionReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_LOAD, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileLoadActionPayload {
    pub vendor: FileVendor,
    pub action: FileLoadAction,
    pub file_name: FixedString<23>,
}
impl Encode for FileLoadActionPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.action as _];
        let string = self.file_name.encode()?;
        encoded.extend(string);

        Ok(encoded)
    }
}
pub type FileMetadataPacket = Cdc2CommandPacket<USER_CDC, FILE_GET_INFO, FileMetadataPayload>;
pub type FileMetadataReplyPacket =
    Cdc2ReplyPacket<USER_CDC, FILE_GET_INFO, Option<FileMetadataReplyPayload>>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadataPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub file_name: FixedString<23>,
}
impl Encode for FileMetadataPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];
        let string = self.file_name.encode()?;
        encoded.extend(string);

        Ok(encoded)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadataReplyPayload {
    /// RESEARCH NEEDED: Unknown what this is if there is no link to the file.
    pub linked_vendor: Option<FileVendor>,
    pub size: u32,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc32: u32,
    pub metadata: FileMetadata,
}
impl Decode for Option<FileMetadataReplyPayload> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let maybe_vid = u8::decode(&mut data).unwrap();

        let linked_vendor = match maybe_vid {
            // 0 is returned if there is no linked file.
            0 => None,
            // 255 is returned if no file was found.
            // In this case, the rest of the packet will be empty, so
            // we return None for the whole packet.
            255 => return Ok(None),
            vid => Some(FileVendor::decode([vid])?),
        };

        let size = u32::decode(&mut data)?;

        // This happens when we try to read a system file from the
        // `/vex_/*` VID. In this case, all of bytes after the vendor
        // will be returned as 0xff or 0x0, making this packet useless,
        // so we'll return `None` here.
        if size == 0xFFFFFFFF {
            return Ok(None);
        }

        let load_address = u32::decode(&mut data)?;
        let crc32 = u32::decode(&mut data)?;
        let metadata = FileMetadata::decode(&mut data)?;

        Ok(Some(FileMetadataReplyPayload {
            linked_vendor,
            size,
            load_address,
            crc32,
            metadata,
        }))
    }
}

pub type FileMetadataSetPacket = Cdc2CommandPacket<USER_CDC, FILE_SET_INFO, FileMetadataSetPayload>;
pub type FileMetadataSetReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_SET_INFO, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadataSetPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub metadata: FileMetadata,
    pub file_name: FixedString<23>,
}
impl Encode for FileMetadataSetPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];
        encoded.extend(self.load_address.to_le_bytes());
        encoded.extend(self.metadata.encode()?);
        encoded.extend(self.file_name.encode()?);
        Ok(encoded)
    }
}

pub type FileErasePacket = Cdc2CommandPacket<USER_CDC, FILE_ERASE, FileErasePayload>;
pub type FileEraseReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_ERASE, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileErasePayload {
    pub vendor: FileVendor,
    /// 128 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub file_name: FixedString<23>,
}
impl Encode for FileErasePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];
        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

pub type FileCleanUpPacket = Cdc2CommandPacket<USER_CDC, FILE_CLEANUP, FileCleanUpPayload>;
pub type FileCleanUpReplyPacket = Cdc2CommandPacket<USER_CDC, FILE_CLEANUP, FileCleanUpResult>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileCleanUpPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}
impl Encode for FileCleanUpPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.vendor as _, self.option])
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// (RESEARCH NEEDED)
pub enum FileCleanUpResult {
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
impl Decode for FileCleanUpResult {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let this = u8::decode(data)?;
        match this {
            0 => Ok(Self::None),
            1 => Ok(Self::AllFiles),
            2 => Ok(Self::LinkedFiles),
            3 => Ok(Self::AllFilesAfterRestart),
            4 => Ok(Self::LinkedFilesAfterRestart),
            value => Err(DecodeError::UnexpectedValue {
                value,
                expected: &[0x00, 0x01, 0x02, 0x03, 0x04],
            }),
        }
    }
}

/// Same as "File Clear Up", but takes longer
pub type FileFormatPacket = Cdc2CommandPacket<USER_CDC, FILE_FORMAT, FileFormatConfirmation>;
pub type FileFormatReplyPacket = Cdc2CommandPacket<USER_CDC, FILE_FORMAT, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileFormatConfirmation {
    /// Must be [0x44, 0x43, 0x42, 0x41].
    pub confirmation_code: [u8; 4],
}
impl Encode for FileFormatConfirmation {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.confirmation_code.to_vec())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileControlGroup {
    Radio(RadioChannel),
}

impl Encode for FileControlGroup {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(match self {
            Self::Radio(channel) => {
                vec![0x01, *channel as u8]
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum RadioChannel {
    // NOTE: There's probably a secret third channel for matches, but that's not known.
    /// Used when controlling the robot outside of a competition match.
    Pit = 0x00,

    /// Used when wirelessly uploading or downloading data to/from the V5 Brain.
    ///
    /// Higher radio bandwidth for file transfer purposes.
    Download = 0x01,
}

pub type FileControlPacket = Cdc2CommandPacket<USER_CDC, FILE_CTRL, FileControlGroup>;
pub type FileControlReplyPacket = Cdc2ReplyPacket<USER_CDC, FILE_CTRL, ()>;
