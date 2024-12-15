use std::{io::Write, time::Duration};

use flate2::{Compression, GzBuilder};
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};

#[cfg(feature = "bluetooth")]
use crate::connection::bluetooth::BluetoothConnection;
use crate::{
    connection::{Connection, ConnectionType},
    crc::VEX_CRC32,
    packets::file::{
        ExitFileTransferPacket, ExitFileTransferReplyPacket, ExtensionType, FileDownloadTarget,
        FileExitAction, FileInitAction, FileInitOption, FileType, FileVendor,
        InitFileTransferPacket, InitFileTransferPayload, InitFileTransferReplyPacket,
        LinkFilePacket, LinkFilePayload, LinkFileReplyPacket, ReadFilePacket, ReadFilePayload,
        ReadFileReplyPacket, WriteFilePacket, WriteFilePayload, WriteFileReplyPacket,
    },
    string::FixedString,
    timestamp::j2000_timestamp,
    version::Version,
};

use super::Command;

pub const COLD_START: u32 = 0x3800000;
pub const HOT_START: u32 = 0x7800000;
const USER_PROGRAM_CHUNK_SIZE: u16 = 4096;

pub struct DownloadFile {
    pub file_name: FixedString<23>,
    pub file_type: FileType,
    pub size: u32,
    pub vendor: FileVendor,
    pub target: Option<FileDownloadTarget>,
    pub load_addr: u32,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for DownloadFile {
    type Output = Vec<u8>;

    async fn execute<C: Connection + ?Sized>(
        mut self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        let target = self.target.unwrap_or(FileDownloadTarget::Qspi);

        let transfer_response = connection
            .packet_handshake::<InitFileTransferReplyPacket>(
                Duration::from_millis(500),
                5,
                InitFileTransferPacket::new(InitFileTransferPayload {
                    operation: FileInitAction::Read,
                    target,
                    vendor: self.vendor,
                    options: FileInitOption::None,
                    write_file_size: self.size,
                    load_address: self.load_addr,
                    write_file_crc: 0,
                    file_type: self.file_type,
                    timestamp: j2000_timestamp(),
                    version: Version {
                        major: 1,
                        minor: 0,
                        build: 0,
                        beta: 0,
                    },
                    file_name: self.file_name,
                }),
            )
            .await?;
        let transfer_response = transfer_response.try_into_inner()?;

        let max_chunk_size = if transfer_response.window_size > 0
            && transfer_response.window_size <= USER_PROGRAM_CHUNK_SIZE
        {
            transfer_response.window_size
        } else {
            USER_PROGRAM_CHUNK_SIZE
        };

        let mut data = Vec::with_capacity(transfer_response.file_size as usize);
        let mut offset = 0;
        loop {
            let read = connection
                .packet_handshake::<ReadFileReplyPacket>(
                    Duration::from_millis(500),
                    5,
                    ReadFilePacket::new(ReadFilePayload {
                        address: self.load_addr + offset,
                        size: max_chunk_size,
                    }),
                )
                .await?;
            let read = read.payload.unwrap()?;
            let chunk_data = read.1;
            offset += chunk_data.len() as u32;
            let last = transfer_response.file_size <= offset;
            let progress = (offset as f32 / transfer_response.file_size as f32) * 100.0;
            data.extend(chunk_data);
            if let Some(callback) = &mut self.progress_callback {
                callback(progress);
            }
            if last {
                break;
            }
        }

        Ok(data)
    }
}

#[cfg(feature = "bluetooth")]
fn max_chunk_size(con_type: ConnectionType, window_size: u16) -> u16 {
    if con_type.is_bluetooth() {
        let max_chunk_size =
            (BluetoothConnection::MAX_PACKET_SIZE as u16).min(window_size / 2) - 14;
        max_chunk_size - (max_chunk_size % 4)
    } else if window_size > 0 && window_size <= USER_PROGRAM_CHUNK_SIZE {
        window_size
    } else {
        USER_PROGRAM_CHUNK_SIZE
    }
}
#[cfg(not(feature = "bluetooth"))]
fn max_chunk_size(_con_type: ConnectionType, window_size: u16) -> u16 {
    if window_size > 0 && window_size <= USER_PROGRAM_CHUNK_SIZE {
        window_size
    } else {
        USER_PROGRAM_CHUNK_SIZE
    }
}

pub struct LinkedFile {
    pub filename: FixedString<23>,
    pub vendor: Option<FileVendor>,
}

pub struct UploadFile<'a> {
    pub filename: FixedString<23>,
    pub file_type: FileType,
    pub vendor: Option<FileVendor>,
    pub data: Vec<u8>,
    pub target: Option<FileDownloadTarget>,
    pub load_addr: u32,
    pub linked_file: Option<LinkedFile>,
    pub after_upload: FileExitAction,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send + 'a>>,
}
impl Command for UploadFile<'_> {
    type Output = ();
    async fn execute<C: Connection + ?Sized>(
        mut self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        info!("Uploading file: {}", self.filename);
        let vendor = self.vendor.unwrap_or(FileVendor::User);
        let target = self.target.unwrap_or(FileDownloadTarget::Qspi);

        let crc = VEX_CRC32.checksum(&self.data);

        let transfer_response = connection
            .packet_handshake::<InitFileTransferReplyPacket>(
                Duration::from_millis(500),
                5,
                InitFileTransferPacket::new(InitFileTransferPayload {
                    operation: FileInitAction::Write,
                    target,
                    vendor,
                    options: FileInitOption::Overwrite,
                    write_file_size: self.data.len() as u32,
                    load_address: self.load_addr,
                    write_file_crc: crc,
                    file_type: self.file_type,
                    timestamp: j2000_timestamp(),
                    version: Version {
                        major: 1,
                        minor: 0,
                        build: 0,
                        beta: 0,
                    },
                    file_name: self.filename.clone(),
                }),
            )
            .await?;
        debug!("transfer init responded");
        let transfer_response = transfer_response.try_into_inner()?;

        if let Some(linked_file) = self.linked_file {
            connection
                .packet_handshake::<LinkFileReplyPacket>(
                    Duration::from_millis(500),
                    5,
                    LinkFilePacket::new(LinkFilePayload {
                        vendor: linked_file.vendor.unwrap_or(FileVendor::User),
                        option: 0,
                        required_file: linked_file.filename,
                    }),
                )
                .await?
                .try_into_inner()?;
        }

        let window_size = transfer_response.window_size;

        // The maximum packet size is 244 bytes for bluetooth
        let max_chunk_size = max_chunk_size(connection.connection_type(), window_size);

        debug!("max_chunk_size: {}", max_chunk_size);

        let mut offset = 0;
        for chunk in self.data.chunks(max_chunk_size as _) {
            let chunk = if chunk.len() < max_chunk_size as _ && chunk.len() % 4 != 0 {
                let mut new_chunk = Vec::new();
                new_chunk.extend_from_slice(chunk);
                new_chunk.resize(chunk.len() + (4 - chunk.len() % 4), 0);
                new_chunk
            } else {
                chunk.to_vec()
            };
            trace!("sending chunk of size: {}", chunk.len());
            let progress = (offset as f32 / self.data.len() as f32) * 100.0;
            if let Some(callback) = &mut self.progress_callback {
                callback(progress);
            }

            let packet = WriteFilePacket::new(WriteFilePayload {
                address: (self.load_addr + offset) as _,
                chunk_data: chunk.clone(),
            });

            // On bluetooth, we dont wait for the reply
            if connection.connection_type() == ConnectionType::Bluetooth {
                connection.send_packet(packet).await?;
            } else {
                connection
                    .packet_handshake::<WriteFileReplyPacket>(Duration::from_millis(500), 5, packet)
                    .await?
                    .try_into_inner()?;
            }

            offset += chunk.len() as u32;
        }
        if let Some(callback) = &mut self.progress_callback {
            callback(100.0);
        }

        connection
            .packet_handshake::<ExitFileTransferReplyPacket>(
                Duration::from_millis(800),
                5,
                ExitFileTransferPacket::new(self.after_upload),
            )
            .await?
            .try_into_inner()?;

        info!("Successfully uploaded file: {}", self.filename.into_inner());
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProgramData {
    #[cfg_attr(feature = "serde_bytes", serde(with = "serde_bytes"))]
    Monolith(Vec<u8>),
    HotCold {
        #[cfg_attr(feature = "serde_bytes", serde(with = "serde_bytes"))]
        hot: Option<Vec<u8>>,

        #[cfg_attr(feature = "serde_bytes", serde(with = "serde_bytes"))]
        cold: Option<Vec<u8>>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Program {
    pub name: String,
    pub slot: u8,
    pub icon: String,
    pub iconalt: String,
    pub description: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Project {
    // version: String,
    pub ide: String,
    // file: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProgramIniConfig {
    pub project: Project,
    pub program: Program,
}

pub struct UploadProgram<'a> {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub program_type: String,
    /// 0-indexed slot
    pub slot: u8,
    pub compress_program: bool,
    pub data: ProgramData,
    pub after_upload: FileExitAction,

    /// Called when progress has been made on the ini file.
    ///
    /// 100.0 should be considered a finished upload.
    pub ini_callback: Option<Box<dyn FnMut(f32) + Send + 'a>>,
    /// Called when progress has been made on the monolith/hot binary
    ///
    /// 100.0 should be considered a finished upload.
    pub bin_callback: Option<Box<dyn FnMut(f32) + Send + 'a>>,
    /// Called when progress has been made on the cold library binary
    ///
    /// 100.0 should be considered a finished upload.
    pub lib_callback: Option<Box<dyn FnMut(f32) + Send + 'a>>,
}
impl Command for UploadProgram<'_> {
    type Output = ();

    async fn execute<C: Connection + ?Sized>(
        mut self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        let base_file_name = format!("slot_{}", self.slot);

        info!("Uploading program ini file");

        let ini = ProgramIniConfig {
            program: Program {
                description: self.description,
                icon: self.icon,
                iconalt: String::new(),
                slot: self.slot,
                name: self.name,
            },
            project: Project {
                ide: self.program_type,
            },
        };

        connection
            .execute_command(UploadFile {
                filename: FixedString::new(format!("{}.ini", base_file_name))?,
                file_type: FileType::new(
                    FixedString::new("ini".to_string())?,
                    ExtensionType::default(),
                ),
                vendor: None,
                data: serde_ini::to_vec(&ini).unwrap(),
                target: None,
                load_addr: COLD_START,
                linked_file: None,
                after_upload: FileExitAction::DoNothing,
                progress_callback: self.ini_callback.take(),
            })
            .await?;

        let program_bin_name = format!("{base_file_name}.bin");
        let program_lib_name = format!("{base_file_name}_lib.bin");

        let is_monolith = matches!(self.data, ProgramData::Monolith(_));
        let (program_data, library_data) = match self.data {
            ProgramData::HotCold { hot, cold } => (hot, cold),
            ProgramData::Monolith(data) => (Some(data), None),
        };

        if let Some(mut library_data) = library_data {
            info!("Uploading cold library binary");

            // Compress the file to improve upload times
            // We don't need to change any other flags, the brain is smart enough to decompress it
            if self.compress_program {
                debug!("Compressing cold library binary");
                compress(&mut library_data);
                debug!("Compression complete");
            }

            connection
                .execute_command(UploadFile {
                    filename: FixedString::new(program_lib_name.clone())?,
                    file_type: FileType::new(
                        FixedString::new("bin".to_string())?,
                        ExtensionType::default(),
                    ),
                    vendor: None,
                    data: library_data,
                    target: None,
                    load_addr: HOT_START,
                    linked_file: None,
                    after_upload: if is_monolith {
                        self.after_upload
                    } else {
                        // we are still uploading, so the post-upload action should not yet be performed
                        FileExitAction::DoNothing
                    },
                    progress_callback: self.lib_callback.take(),
                })
                .await?;
        }

        if let Some(mut program_data) = program_data {
            info!("Uploading program binary");

            if self.compress_program {
                debug!("Compressing program binary");
                compress(&mut program_data);
                debug!("Compression complete");
            }

            // Only ask the brain to link to a library if the program expects it.
            // Monolith programs don't have libraries.
            let linked_file = if is_monolith {
                None
            } else {
                info!("Program will be linked to cold library: {program_lib_name:?}");
                Some(LinkedFile {
                    filename: FixedString::new(program_lib_name)?,
                    vendor: None,
                })
            };

            connection
                .execute_command(UploadFile {
                    filename: FixedString::new(program_bin_name)?,
                    file_type: FileType::new(
                        FixedString::new("bin".to_string())?,
                        ExtensionType::Binary,
                    ),
                    vendor: None,
                    data: program_data,
                    target: None,
                    load_addr: COLD_START,
                    linked_file,
                    after_upload: self.after_upload,
                    progress_callback: self.bin_callback.take(),
                })
                .await?;
        }

        Ok(())
    }
}

/// Apply gzip compression to the given data
fn compress(data: &mut Vec<u8>) {
    let mut encoder = GzBuilder::new().write(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    *data = encoder.finish().unwrap();
}
