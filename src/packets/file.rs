use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use super::Version;

#[repr(u8)]
pub enum FileInitAction {
    Write = 1,
    Read = 2,
}

#[repr(u8)]
pub enum FileInitOption {
    None = 0,
    Overwrite = 1,
}

#[repr(u8)]
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

#[repr(u8)]
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
    pub vendor: u8,
    pub options: FileInitOption,
    pub write_file_size: u32,
    pub load_address: u32,
    pub write_file_crc: crc::Algorithm<u32>,
    pub file_extension: String,
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
    pub file_crc: crc::Algorithm<u32>,
}

/// Finish uploading or downloading file from the device
pub type ExitFileTransferPacket = Cdc2CommandPacket<0x56, 0x12, FileExitAtion>;
pub type ExitFileTransferReplyPacket = Cdc2ReplyPacket<0x56, 0x12, ()>;

/// The action to run when a file transfer is completed.
#[repr(u8)]
pub enum FileExitAtion {
    DoNothing = 0,
    RunProgram = 1,
    ShowRunScreen = 2,
    Halt = 3,
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

/// Read from the brain
pub type ReadFilePacket = Cdc2CommandPacket<0x56, 0x14, ReadFilePayload>;
/// Returns the file content. This packet doesn't have an ack if the data is available.
pub type ReadFileReplyPacket = Cdc2ReplyPacket<0x56, 0x14, ReadFileReplyPayload>;

pub struct ReadFilePayload {
    /// Memory address to read from.
    pub address: u32,

    /// Number of bytes to read (4-byte aligned).
    pub size: u16,
}

pub struct ReadFileReplyPayload {
    /// Memory address to read from.
    pub address: u32,

    /// The data from the brain
    pub chunk_data: Vec<u8>,
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
    pub required_file: String,
}

pub type GetDirectoryFileCountPacket = Cdc2CommandPacket<0x56, 0x16, GetDirectoryFileCountPayload>;
pub type GetDirectoryFileCountReplyPacket = Cdc2ReplyPacket<0x56, 0x16, u16>;

pub struct GetDirectoryFileCountPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}

pub type GetDirectoryEntryPacket = Cdc2CommandPacket<0x56, 0x17, GetDirectoryEntryPayload>;
pub type GetDirectoryEntryReplyPacket =
    Cdc2ReplyPacket<0x56, 0x17, Option<GetDirectoryEntryReplyPayload>>;

pub struct GetDirectoryEntryPayload {
    pub file_index: u8,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}

pub struct GetDirectoryEntryReplyPayload {
    pub file_index: u8,
    pub size: u32,

    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc32: crc::Algorithm<u32>,
    pub file_type: String,

    /// The unix epoch timestamp minus [`J2000_EPOCH`].
    pub timestamp: i32,
    pub version: Version,
    pub file_name: String,
}

/// Run a binrary file on the brain or stop the program running on the brain.
pub type LoadFileActionPacket = Cdc2CommandPacket<0x56, 0x18, LoadFileActionPayload>;
pub type LoadFileActionReplyPacket = Cdc2ReplyPacket<0x56, 0x18, ()>;

pub struct LoadFileActionPayload {
    pub vendor: FileVendor,
    pub action: FileInitAction,
    pub file_name: String,
}

pub type GetFileMetadataPacket = Cdc2CommandPacket<0x56, 0x19, GetFileMetadataPayload>;
pub type GetFileMetadataReplyPacket =
    Cdc2ReplyPacket<0x56, 0x19, Option<GetFileMetadataReplyPayload>>;

pub struct GetFileMetadataPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub file_name: String,
}

pub struct GetFileMetadataReplyPayload {
    pub ignored: u8,
    pub size: u32,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc32: crc::Algorithm<u32>,
    pub file_type: String,
    /// The unix epoch timestamp minus [`J2000_EPOCH`].
    pub timestamp: i32,
    pub version: Version,
}

pub type SetFileMetadataPacket = Cdc2CommandPacket<0x56, 0x1a, SetFileMetadataPayload>;
pub type SetFileMetadataReplyPacket = Cdc2ReplyPacket<0x56, 0x1a, ()>;

pub struct SetFileMetadataPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub file_type: String,
    /// The unix epoch timestamp minus J2000_EPOCH.
    pub timestamp: i32,
    pub version: Version,
    pub file_name: String,
}

pub type EraseFilePacket = Cdc2CommandPacket<0x56, 0x1b, EraseFilePayload>;
pub type EraseFileReplyPacket = Cdc2ReplyPacket<0x56, 0x1b, ()>;

pub struct EraseFilePayload {
    pub vendor: FileVendor,
    /// 128 = default. (RESEARCH NEEDED)
    pub option: u8,
    pub file_name: String,
}

pub type FileClearUpPacket = Cdc2CommandPacket<0x56, 0x1e, FileClearUpPayload>;
pub type FileClearUpReplyPacket = Cdc2CommandPacket<0x56, 0x1e, FileClearUpResult>;

pub struct FileClearUpPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
}

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

/// Same as "File Clear Up", but takes longer
pub type FileFormatPacket = Cdc2CommandPacket<0x56, 0x1f, FileFormatConfirmation>;
pub type FileFormatReplyPacket = Cdc2CommandPacket<0x56, 0x1f, ()>;

pub struct FileFormatConfirmation {
    /// Must be [0x44, 0x43, 0x42, 0x41].
    pub confirmation_code: [u8; 4],
}
