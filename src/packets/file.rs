//! Filesystem Access

use std::str;

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
    decode::{Decode, DecodeError, SizedDecode},
    encode::Encode,
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
    fn size(&self) -> usize {
        12
    }

    fn encode(&self, data: &mut [u8]) {
        let extension = self.extension.as_ref();
        data[..extension.len()].copy_from_slice(extension.as_bytes());
        data[3] = self.extension_type as _;
        self.timestamp.encode(&mut data[4..]);
        self.version.encode(&mut data[8..]);
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
    fn size(&self) -> usize {
        28 + self.file_name.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.operation as _;
        data[1] = self.target as _;
        data[2] = self.vendor as _;
        data[3] = self.options as _;
        self.file_size.encode(&mut data[4..]);
        self.load_address.encode(&mut data[8..]);
        self.write_file_crc.encode(&mut data[12..]);
        self.metadata.encode(&mut data[16..]);
        self.file_name.encode(&mut data[28..]);
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
    fn size(&self) -> usize {
        1
    }
    fn encode(&self, data: &mut [u8]) {
        data[0] = *self as _;
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
    fn size(&self) -> usize {
        4 + self.chunk_data.len()
    }
    
    fn encode(&self, data: &mut [u8]) {
        self.address.encode(data);
        self.chunk_data.encode(&mut data[4..]);
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
    fn size(&self) -> usize {
        6
    }

    fn encode(&self, data: &mut [u8]) {
        self.address.encode(data);
        self.size.encode(&mut data[4..]);
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FileDataReadReplyContents {
    Ack {
        address: u32,
        data: Vec<u8>,
    },
    Nack(Cdc2Ack),
}

impl SizedDecode for FileDataReadReplyContents {
    fn sized_decode(data: impl IntoIterator<Item = u8>, size: u16) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        if size == 1 {
            Ok(Self::Nack(Cdc2Ack::decode(&mut data)?))
        } else {
            let address = u32::decode(&mut data)?;

            let chunk_data = Vec::sized_decode(&mut data, size - 4)?;
            Ok(Self::Ack {
                address,
                data: chunk_data,
            })
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileDataReadReplyPayload {
    pub contents: FileDataReadReplyContents,
    pub crc: u16,
}
impl SizedDecode for FileDataReadReplyPayload {
    fn sized_decode(data: impl IntoIterator<Item = u8>, size: u16) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let ecmd = u8::decode(&mut data)?;
        if ecmd != FILE_READ {
            return Err(DecodeError::UnexpectedValue {
                value: ecmd,
                expected: &[FILE_READ],
            });
        }

        let contents = FileDataReadReplyContents::sized_decode(&mut data, size - 3)?;
        let crc = u16::decode(&mut data)?.swap_bytes();

        Ok(Self { contents, crc })
    }
}
impl FileDataReadReplyPayload {
    pub fn unwrap(self) -> Result<(u32, Vec<u8>), Cdc2Ack> {
        match self.contents {
            FileDataReadReplyContents::Ack {
                address,
                data,
            } => Ok((address, data)),
            FileDataReadReplyContents::Nack(nack) => Err(nack),
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
    fn size(&self) -> usize {
        2 + self.required_file.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.option;
        self.required_file.encode(&mut data[2..]);
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
    fn size(&self) -> usize {
        2
    }
    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.option;
    }
}

pub type DirectoryEntryPacket = Cdc2CommandPacket<USER_CDC, FILE_DIR_ENTRY, DirectoryEntryPayload>;
pub type DirectoryEntryReplyPacket =
    Cdc2ReplyPacket<USER_CDC, FILE_DIR_ENTRY, Option<DirectoryEntryReplyPayload>>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DirectoryEntryPayload {
    pub file_index: u8,
    /// 0 = default. (RESEARCH NEEDED)
    pub unknown: u8,
}
impl Encode for DirectoryEntryPayload {
    fn size(&self) -> usize {
        2
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.file_index;
        data[1] = self.unknown;
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
    fn size(&self) -> usize {
        2 + self.file_name.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.action as _;
        self.file_name.encode(&mut data[2..]);
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
    fn size(&self) -> usize {
        2 + self.file_name.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.option as _;
        self.file_name.encode(&mut data[2..]);
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
    fn size(&self) -> usize {
        18 + self.file_name.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.option as _;
        self.load_address.encode(&mut data[2..]);
        self.metadata.encode(&mut data[6..]);
        self.file_name.encode(&mut data[18..]);
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
    fn size(&self) -> usize {
        2 + self.file_name.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.option as _;
        self.file_name.encode(&mut data[2..]);
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
    fn size(&self) -> usize {
        2
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.option as _;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
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

impl FileFormatConfirmation {
    pub const FORMAT_CODE: [u8; 4] = [0x44, 0x43, 0x42, 0x41];

    pub const fn new() -> Self {
        Self {
            confirmation_code: Self::FORMAT_CODE,
        }
    }
}

impl Default for FileFormatConfirmation {
    fn default() -> Self {
        Self::new()
    }
}

impl Encode for FileFormatConfirmation {
    fn size(&self) -> usize {
        4
    }
    
    fn encode(&self, data: &mut [u8]) {
        self.confirmation_code.encode(data)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileControlGroup {
    Radio(RadioChannel),
}

impl Encode for FileControlGroup {
    fn size(&self) -> usize {
        if matches!(self, Self::Radio(_)) {
            2
        } else {
            0
        }
    }

    fn encode(&self, data: &mut [u8]) {
        #[allow(irrefutable_let_patterns)] // may change in the future
        if let Self::Radio(channel) = self {
            data[0] = 0x01;
            data[1] = *channel as _;
        }
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
