//! Internal filesystem access packets.

use core::str;

use alloc::vec::Vec;

use crate::{
    Decode, DecodeError, DecodeWithLength, Encode, FixedString, Version,
    cdc::{CdcCommand, CdcReply, cmds, decode_cdc_reply_frame},
    cdc2::{Cdc2Ack, Cdc2Command, Cdc2Reply, cdc2_command_size, ecmds, frame_cdc2_command},
    cdc2_pair,
    decode::DecodeErrorKind,
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
pub enum FileTransferOptions {
    None = 0,
    Overwrite = 1,
    EraseAll = 0x80,
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
#[non_exhaustive]
pub enum FileVendor {
    // V5/General user programs
    User = 0x01,
    Sys = 0x0F,
    Dev1 = 0x10,
    Dev2 = 0x18,
    Dev3 = 0x20,
    Dev4 = 0x28,
    Dev5 = 0x30,
    Dev6 = 0x38,
    VexVm = 0x40,
    Vex = 0xF0,
    Undefined = 0xF1,

    /// Used to VEX AIR .vexos packages.
    VexAirFirmware = 0x02,
    
    /// Used for VEX AIR python packages.
    VexAirVm = 0x03,

    /// VEX air mission files (normally mounted readonly as mass storage).
    VexAirMissions = 0x04,

    /// AIM image.
    AimImage = 0x80,

    /// AIM audio asset.
    AimSound = 0x88,

    /// This vendor is used for firmware updates on the AIM radio, which is an esp32s3.
    AimRadio = 0xFC,
}
impl Decode for FileVendor {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        match u8::decode(data)? {
            0x01 => Ok(Self::User),
            0x02 => Ok(Self::VexAirFirmware),
            0x03 => Ok(Self::VexAirVm),
            0x0F => Ok(Self::Sys),
            0x10 => Ok(Self::Dev1),
            0x18 => Ok(Self::Dev2),
            0x20 => Ok(Self::Dev3),
            0x28 => Ok(Self::Dev4),
            0x30 => Ok(Self::Dev5),
            0x38 => Ok(Self::Dev6),
            0x40 => Ok(Self::VexVm),
            0xF0 => Ok(Self::Vex),
            0xF1 => Ok(Self::Undefined),
            0x80 => Ok(Self::AimImage),
            0x88 => Ok(Self::AimSound),
            0xFC => Ok(Self::AimRadio),
            v => Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                name: "FileVendor",
                value: v,
                expected: &[
                    0x01, 0x02, 0x03, 0x0F, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x40, 0xF0, 0xF1,
                    0x80, 0x88, 0xFC,
                ],
            })),
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
#[non_exhaustive]
pub enum ExtensionType {
    /// Regular unencrypted file.
    #[default]
    Binary = 0x0,

    /// A file which depends on a VM.
    ///
    /// This is the file type used for VEXCode Python bin uploads, since they need the
    /// Python VM to function.
    Vm = 0x61,

    /// File's contents is encrypted.
    EncryptedBinary = 0x73,

    /// Zipped program binary (VEX AIR only).
    Zipped = 0x7A,
}

impl Decode for ExtensionType {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(match u8::decode(data)? {
            0x0 => Self::Binary,
            0x61 => Self::Vm,
            0x73 => Self::EncryptedBinary,
            0x7A => Self::Zipped,
            unknown => {
                return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                    name: "ExtensionType",
                    value: unknown,
                    expected: &[0x0, 0x61, 0x73, 0x7A],
                }));
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
        data[..self.extension.len()].copy_from_slice(self.extension.as_bytes());
        data[3] = self.extension_type as _;
        self.timestamp.encode(&mut data[4..]);
        self.version.encode(&mut data[8..]);
    }
}

impl Decode for FileMetadata {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            // SAFETY: length is guaranteed to be less than 4.
            extension: unsafe {
                FixedString::new_unchecked(
                    str::from_utf8(&<[u8; 3]>::decode(data)?)
                        .map_err(|e| DecodeError::new::<Self>(e.into()))?,
                )
            },
            extension_type: Decode::decode(data).unwrap(),
            timestamp: i32::decode(data)?,
            version: Version::decode(data)?,
        })
    }
}

// MARK: FileTransferInitialize

cdc2_pair!(
    FileTransferInitializePacket => FileTransferInitializeReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_INIT,
);

/// Start uploading or downloading file from the device
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileTransferInitializePacket {
    pub operation: FileTransferOperation,
    pub target: FileTransferTarget,
    pub vendor: FileVendor,
    pub options: FileTransferOptions,
    pub file_size: u32,
    pub load_address: u32,
    pub write_file_crc: u32,
    pub metadata: FileMetadata,
    pub file_name: FixedString<23>,
}

impl Encode for FileTransferInitializePacket {
    fn size(&self) -> usize {
        cdc2_command_size(28 + self.file_name.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            [
                self.operation as u8,
                self.target as u8,
                self.vendor as u8,
                self.options as u8,
            ]
            .encode(data);
            self.file_size.encode(&mut data[4..]);
            self.load_address.encode(&mut data[8..]);
            self.write_file_crc.encode(&mut data[12..]);
            self.metadata.encode(&mut data[16..]);
            self.file_name.encode(&mut data[28..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileTransferInitializeReplyPacket {
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

impl Decode for FileTransferInitializeReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            window_size: Decode::decode(data)?,
            file_size: Decode::decode(data)?,
            // Convert from big endian
            file_crc: u32::decode(data)?.swap_bytes(),
        })
    }
}

// MARK: FileTransferExit

cdc2_pair!(
    FileTransferExitPacket => FileTransferExitReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_EXIT,
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileTransferExitPacket {
    pub action: FileExitAction,
}

impl Encode for FileTransferExitPacket {
    fn size(&self) -> usize {
        cdc2_command_size(1)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.action.encode(data);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileTransferExitReplyPacket {}

impl Decode for FileTransferExitReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

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

// MARK: FileDataWritePacket

cdc2_pair!(
    FileDataWritePacket => FileDataWriteReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_WRITE,
);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileDataWritePacket {
    /// Memory address to write to.
    pub address: i32,

    /// A sequence of bytes to write. Must be 4-byte aligned.
    pub chunk_data: Vec<u8>,
}

impl Encode for FileDataWritePacket {
    fn size(&self) -> usize {
        cdc2_command_size(4 + self.chunk_data.len())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.address.encode(data);
            self.chunk_data.encode(&mut data[4..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileDataWriteReplyPacket {}

impl Decode for FileDataWriteReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: FileDataRead

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileDataReadPacket {
    /// Memory address to read from.
    pub address: u32,

    /// Number of bytes to read (4-byte aligned).
    pub size: u16,
}

impl Encode for FileDataReadPacket {
    fn size(&self) -> usize {
        cdc2_command_size(6)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.address.encode(data);
            self.size.encode(&mut data[4..]);
        });
    }
}

impl CdcCommand for FileDataReadPacket {
    type Reply = Result<FileDataReadReplyPacket, Cdc2Ack>;
    const CMD: u8 = cmds::USER_CDC;
}

impl Cdc2Command for FileDataReadPacket {
    const ECMD: u8 = ecmds::FILE_READ;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileDataReadReplyPacket {
    pub address: u32,
    pub data: Vec<u8>,
}

impl Decode for FileDataReadReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let address = u32::decode(data)?;
        let chunk_data = Vec::decode_with_len(data, data.len())?;

        Ok(Self {
            address,
            data: chunk_data,
        })
    }
}

impl Decode for Result<FileDataReadReplyPacket, Cdc2Ack> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        decode_cdc_reply_frame::<Self>(data)?;

        let ecmd = u8::decode(data)?;
        if ecmd != ecmds::FILE_READ {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                name: "ecmd",
                value: ecmd,
                expected: &[ecmds::FILE_READ],
            }));
        }

        let payload_data = &mut data
            .get(..data.len() - 2)
            .ok_or_else(|| DecodeError::new::<Self>(DecodeErrorKind::UnexpectedEnd))?;

        let payload = Ok(if payload_data.len() == 1 {
            Err(Cdc2Ack::decode(payload_data)?)
        } else {
            Ok(FileDataReadReplyPacket::decode(payload_data)?)
        });

        *data = &data[data.len() - 2..];

        let _crc = u16::decode(data)?.swap_bytes();

        payload
    }
}

impl CdcReply for Result<FileDataReadReplyPacket, Cdc2Ack> {
    type Command = FileDataReadPacket;
    const CMD: u8 = cmds::USER_CDC;
}

impl Cdc2Reply for Result<FileDataReadReplyPacket, Cdc2Ack> {
    const ECMD: u8 = ecmds::FILE_READ;
}

// MARK: FileLink

cdc2_pair!(
    FileLinkPacket => FileLinkReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_LINK,
);

/// File linking means allowing one file to be loaded after another file first (its parent).
///
/// This is used in PROS for the hot/cold linking.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileLinkPacket {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub reserved: u8,
    pub required_file: FixedString<23>,
}

impl Encode for FileLinkPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2 + self.required_file.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.reserved;
            self.required_file.encode(&mut data[2..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileLinkReplyPacket {}

impl Decode for FileLinkReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: DirectoryFileCount

cdc2_pair!(
    DirectoryFileCountPacket => DirectoryFileCountReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_DIR,
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DirectoryFileCountPacket {
    pub vendor: FileVendor,
    /// Unused as of VEXos 1.1.5
    pub reserved: u8,
}

impl Encode for DirectoryFileCountPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.reserved;
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DirectoryFileCountReplyPacket {
    pub count: u16,
}

impl Decode for DirectoryFileCountReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            count: u16::decode(data)?,
        })
    }
}

// MARK: DirectoryEntry

cdc2_pair!(
    DirectoryEntryPacket => DirectoryEntryReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_DIR_ENTRY,
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DirectoryEntryPacket {
    pub file_index: u8,
    pub reserved: u8,
}

impl Encode for DirectoryEntryPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.file_index;
            data[1] = self.reserved;
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntryReplyPacket {
    pub file_index: u8,
    pub size: u32,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc: u32,
    pub metadata: Option<FileMetadata>,
    pub file_name: FixedString<23>,
}

impl Decode for DirectoryEntryReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let file_index = u8::decode(data)?;
        let size = u32::decode(data)?;
        let load_address = u32::decode(data)?;
        let crc = u32::decode(data)?;

        let metadata = if data.get(0) == Some(&255) {
            let _ = <[u8; 12]>::decode(data);
            None
        } else {
            Some(FileMetadata::decode(data)?)
        };

        let file_name = FixedString::<23>::decode(data)?;

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

// MARK: FileLoadAction

cdc2_pair!(
    FileLoadActionPacket => FileLoadActionReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_LOAD,
);

/// Run a binary file on the brain or stop the program running on the brain.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileLoadActionPacket {
    pub vendor: FileVendor,
    pub action: FileLoadAction,
    pub file_name: FixedString<23>,
}

impl Encode for FileLoadActionPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2 + self.file_name.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.action as _;
            self.file_name.encode(&mut data[2..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileLoadActionReplyPacket {}

impl Decode for FileLoadActionReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: FileMetadata

cdc2_pair!(
    FileMetadataPacket => Option<FileMetadataReplyPacket>,
    cmds::USER_CDC,
    ecmds::FILE_GET_INFO,
);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadataPacket {
    pub vendor: FileVendor,
    /// Unused as of VEXos 1.1.5
    pub reserved: u8,
    pub file_name: FixedString<23>,
}

impl Encode for FileMetadataPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2 + self.file_name.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.reserved;
            self.file_name.encode(&mut data[2..]);
        });
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadataReplyPacket {
    /// RESEARCH NEEDED: Unknown what this is if there is no link to the file.
    pub linked_vendor: Option<FileVendor>,
    pub size: u32,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub crc32: u32,
    pub metadata: FileMetadata,
}

impl Decode for Option<FileMetadataReplyPacket> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let maybe_vid = u8::decode(data)?;

        let linked_vendor = match maybe_vid {
            // 0 is returned if there is no linked file.
            0 => None,
            // 255 is returned if no file was found.
            // In this case, the rest of the packet will be empty, so
            // we return None for the whole packet.
            255 => return Ok(None),
            vid => Some(FileVendor::decode(&mut [vid].as_slice())?),
        };

        let size = u32::decode(data)?;

        // This happens when we try to read a system file from the
        // `/vex_/*` VID. In this case, all of bytes after the vendor
        // will be returned as 0xff or 0x0, making this packet useless,
        // so we'll return `None` here.
        if size == 0xFFFFFFFF {
            return Ok(None);
        }

        let load_address = u32::decode(data)?;
        let crc32 = u32::decode(data)?;
        let metadata = FileMetadata::decode(data)?;

        Ok(Some(FileMetadataReplyPacket {
            linked_vendor,
            size,
            load_address,
            crc32,
            metadata,
        }))
    }
}

// MARK: FileMetadataSet

cdc2_pair!(
    FileMetadataSetPacket => FileMetadataSetReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_SET_INFO,
);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileMetadataSetPacket {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub options: u8,
    /// The storage entry address of the file.
    pub load_address: u32,
    pub metadata: FileMetadata,
    pub file_name: FixedString<23>,
}

impl Encode for FileMetadataSetPacket {
    fn size(&self) -> usize {
        cdc2_command_size(18 + self.file_name.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.options;
            self.load_address.encode(&mut data[2..]);
            self.metadata.encode(&mut data[6..]);
            self.file_name.encode(&mut data[18..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileMetadataSetReplyPacket {}

impl Decode for FileMetadataSetReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: FileErase

cdc2_pair!(
    FileErasePacket => FileEraseReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_ERASE,
);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileErasePacket {
    pub vendor: FileVendor,
    /// Unused as of VEXos 1.1.5
    pub reserved: u8,
    pub file_name: FixedString<23>,
}

impl Encode for FileErasePacket {
    fn size(&self) -> usize {
        cdc2_command_size(2 + self.file_name.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.reserved;
            self.file_name.encode(&mut data[2..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileEraseReplyPacket {}

impl Decode for FileEraseReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: FileCleanUp

cdc2_pair!(
    FileCleanUpPacket => FileCleanUpReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_CLEANUP,
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileCleanUpPacket {}

impl Encode for FileCleanUpPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileCleanUpReplyPacket {
    pub count: u16,
}

impl Decode for FileCleanUpReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            count: u16::decode(data)?,
        })
    }
}

// MARK: FileFormat

cdc2_pair!(
    FileFormatPacket => FileFormatReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_FORMAT,
);

/// Same as "File Clean Up", but takes longer
pub struct FileFormatPacket {
    /// Must be [0x44, 0x43, 0x42, 0x41].
    pub confirmation_code: [u8; 4],
}

impl FileFormatPacket {
    pub const FORMAT_CODE: [u8; 4] = [0x44, 0x43, 0x42, 0x41];

    pub const fn new() -> Self {
        Self {
            confirmation_code: Self::FORMAT_CODE,
        }
    }
}

impl Default for FileFormatPacket {
    fn default() -> Self {
        Self::new()
    }
}

impl Encode for FileFormatPacket {
    fn size(&self) -> usize {
        cdc2_command_size(4)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.confirmation_code.encode(data);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileFormatReplyPacket {}

impl Decode for FileFormatReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

/// MARK: FileControlPacket

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FileControlGroup {
    Radio(RadioChannel),
}

impl Encode for FileControlGroup {
    fn size(&self) -> usize {
        if matches!(self, Self::Radio(_)) { 2 } else { 0 }
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

// MARK: FileControl

cdc2_pair!(
    FileControlPacket => FileControlReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_CTRL,
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileControlPacket {
    pub group: FileControlGroup,
}

impl Encode for FileControlPacket {
    fn size(&self) -> usize {
        cdc2_command_size(self.group.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.group.encode(data);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FileControlReplyPacket {}

impl Decode for FileControlReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: ProgramStatus

cdc2_pair!(
    ProgramStatusPacket => ProgramStatusReplyPacket,
    cmds::USER_CDC,
    ecmds::FILE_USER_STAT,
);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProgramStatusPacket {
    pub vendor: FileVendor,
    /// Unused as of VEXos 1.1.5
    pub reserved: u8,
    /// The bin file name.
    pub file_name: FixedString<23>,
}

impl Encode for ProgramStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2 + self.file_name.size())
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.vendor as _;
            data[1] = self.reserved;
            self.file_name.encode(&mut data[2..]);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ProgramStatusReplyPacket {
    /// A zero-based slot number.
    pub slot: u8,
    /// A zero-based slot number, always same as Slot.
    pub requested_slot: u8,
}

impl Decode for ProgramStatusReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            slot: u8::decode(data)?,
            requested_slot: u8::decode(data)?,
        })
    }
}
