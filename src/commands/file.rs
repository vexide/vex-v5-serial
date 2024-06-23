use std::{io::Write, time::Duration};

use flate2::{Compression, GzBuilder};
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};

use crate::{
    connection::{serial::SerialConnection, ConnectionError},
    crc::VEX_CRC32,
    packets::file::{
            ExitFileTransferPacket, ExitFileTransferReplyPacket, FileDownloadTarget, FileExitAtion,
            FileInitAction, FileInitOption, FileVendor, InitFileTransferPacket,
            InitFileTransferPayload, InitFileTransferReplyPacket, LinkFilePacket, LinkFilePayload,
            LinkFileReplyPacket, ReadFilePacket, ReadFilePayload, ReadFileReplyPacket,
            WriteFilePacket, WriteFilePayload, WriteFileReplyPacket,
        },
    string::FixedLengthString,
    timestamp::j2000_timestamp,
    version::Version,
};

use super::Command;

pub const COLD_START: u32 = 0x3800000;
const USER_PROGRAM_CHUNK_SIZE: u16 = 4096;

pub struct DownloadFile {
    pub filename: FixedLengthString<23>,
    pub filetype: FixedLengthString<3>,
    pub size: u32,
    pub vendor: FileVendor,
    pub target: Option<FileDownloadTarget>,
    pub load_addr: u32,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for DownloadFile {
    type Output = Vec<u8>;

    async fn execute(
        &mut self,
        connection: &mut SerialConnection,
    ) -> Result<Self::Output, ConnectionError> {
        let target = self.target.unwrap_or(FileDownloadTarget::Qspi);

        let transfer_response = connection
            .packet_handshake::<InitFileTransferReplyPacket>(
                Duration::from_millis(100),
                5,
                InitFileTransferPacket::new(InitFileTransferPayload {
                    operation: FileInitAction::Read,
                    target,
                    vendor: self.vendor,
                    options: FileInitOption::None,
                    write_file_size: self.size,
                    load_address: self.load_addr,
                    write_file_crc: 0,
                    file_extension: self.filetype.clone(),
                    timestamp: j2000_timestamp(),
                    version: Version {
                        major: 1,
                        minor: 0,
                        build: 0,
                        beta: 0,
                    },
                    file_name: self.filename.clone(),
                })
            )
            .await?;
        let transfer_response = transfer_response.payload.try_into_inner()?;

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
                    Duration::from_millis(100),
                    5,
                    ReadFilePacket::new(ReadFilePayload {
                        address: self.load_addr + offset,
                        size: max_chunk_size,
                    }),
                )
                .await?;
            let read = read.payload.unwrap().map_err(ConnectionError::Nack)?;
            let chunk_data = read.1.into_inner();
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

pub struct LinkedFile {
    pub filename: FixedLengthString<23>,
    pub vendor: Option<FileVendor>,
}

pub struct UploadFile {
    pub filename: FixedLengthString<23>,
    pub filetype: FixedLengthString<3>,
    pub vendor: Option<FileVendor>,
    pub data: Vec<u8>,
    pub target: Option<FileDownloadTarget>,
    pub load_addr: u32,
    pub linked_file: Option<LinkedFile>,
    pub after_upload: FileExitAtion,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for UploadFile {
    type Output = ();
    async fn execute(
        &mut self,
        connection: &mut SerialConnection,
    ) -> Result<Self::Output, ConnectionError> {
        info!("Uploading file: {}", self.filename);
        let vendor = self.vendor.unwrap_or(FileVendor::User);
        let target = self.target.unwrap_or(FileDownloadTarget::Qspi);

        let crc = VEX_CRC32.checksum(&self.data);

        let transfer_response = connection
            .packet_handshake::<InitFileTransferReplyPacket>(
                Duration::from_millis(100),
                5,
                InitFileTransferPacket::new(InitFileTransferPayload {
                    operation: FileInitAction::Write,
                    target,
                    vendor,
                    options: FileInitOption::Overwrite,
                    write_file_size: self.data.len() as u32,
                    load_address: self.load_addr,
                    write_file_crc: crc,
                    file_extension: self.filetype.clone(),
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
        let transfer_response = transfer_response.payload.try_into_inner()?;

        if let Some(linked_file) = &self.linked_file {
            connection
                .packet_handshake::<LinkFileReplyPacket>(
                    Duration::from_millis(100),
                    5,
                    LinkFilePacket::new(LinkFilePayload {
                        vendor: linked_file.vendor.unwrap_or(FileVendor::User),
                        option: 0,
                        required_file: linked_file.filename.clone(),
                    }),
                )
                .await?
                .payload
                .try_into_inner()?;
        }

        let window_size = transfer_response.window_size;
        let max_chunk_size = if window_size > 0 && window_size <= USER_PROGRAM_CHUNK_SIZE {
            window_size
        } else {
            USER_PROGRAM_CHUNK_SIZE
        };

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

            connection
                .packet_handshake::<WriteFileReplyPacket>(
                    Duration::from_millis(100),
                    5,
                    WriteFilePacket::new(WriteFilePayload {
                        address: (self.load_addr + offset) as _,
                        chunk_data: chunk.clone(),
                    }),
                )
                .await?
                .payload
                .try_into_inner()?;
            offset += chunk.len() as u32;
        }
        if let Some(callback) = &mut self.progress_callback {
            callback(100.0);
        }

        connection
            .packet_handshake::<ExitFileTransferReplyPacket>(
                Duration::from_millis(100),
                5,
                ExitFileTransferPacket::new(self.after_upload),
            )
            .await?
            .payload
            .try_into_inner()?;

        info!("Successfully uploaded file: {}", self.filename);
        Ok(())
    }
}

#[derive(Debug)]
pub enum ProgramData {
    Hot(Vec<u8>),
    Cold(Vec<u8>),
    Both { hot: Vec<u8>, cold: Vec<u8> },
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

#[derive(Debug)]
pub struct UploadProgram {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub program_type: String,
    /// 0-indexed slot
    pub slot: u8,
    pub compress_program: bool,
    pub data: ProgramData,
    pub after_upload: FileExitAtion,
}
impl Command for UploadProgram {
    type Output = ();

    async fn execute(&mut self, connection: &mut SerialConnection) -> Result<Self::Output, ConnectionError> {
        let base_file_name = format!("slot{}", self.slot);

        let ini = ProgramIniConfig {
            program: Program {
                description: self.description.clone(),
                icon: self.icon.clone(),
                iconalt: String::new(),
                slot: self.slot,
                name: self.name.clone(),
            },
            project: Project {
                ide: self.program_type.clone(),
            },
        };
        let ini = serde_ini::to_vec(&ini).unwrap();

        let file_transfer = UploadFile {
            filename: FixedLengthString::new(format!("{}.ini", base_file_name))?,
            filetype: FixedLengthString::new("ini".to_string())?,
            vendor: None,
            data: ini,
            target: None,
            load_addr: COLD_START,
            linked_file: None,
            after_upload: FileExitAtion::Halt,
            progress_callback: Some(Box::new(|progress| {
                info!("Uploading INI: {:.2}%", progress)
            })),
        };
        connection.execute_command(file_transfer).await.unwrap();

        let (cold, hot) = match &mut self.data {
            ProgramData::Cold(cold) => (Some(cold), None),
            ProgramData::Hot(hot) => (None, Some(hot)),
            ProgramData::Both { hot, cold } => (Some(cold), Some(hot)),
        };

        if let Some(cold) = cold {
            info!("Uploading cold binary");
            let after_upload = if hot.is_some() {
                FileExitAtion::Halt
            } else {
                self.after_upload
            };

            // Compress the cold program
            // We don't need to change any other flags, the brain is smart enough to decompress it
            if self.compress_program {
                debug!("Compressing cold binary");
                let compressed = Vec::new();
                let mut encoder = GzBuilder::new().write(compressed, Compression::default());
                encoder.write_all(cold).unwrap();
                *cold = encoder.finish().unwrap();
            }

            connection
                .execute_command(UploadFile {
                    filename: FixedLengthString::new(format!("{}.bin", base_file_name))?,
                    filetype: FixedLengthString::new("bin".to_string())?,
                    vendor: None,
                    data: cold.clone(),
                    target: None,
                    load_addr: COLD_START,
                    linked_file: None,
                    after_upload,
                    progress_callback: Some(Box::new(|progress| {
                        info!("Uploading cold: {:.2}%", progress)
                    })),
                })
                .await?;
        }

        if let Some(hot) = hot {
            info!("Uploading hot binary");
            let linked_file = Some(LinkedFile {
                filename: FixedLengthString::new(format!("{}_lib.bin", base_file_name))?,
                vendor: None,
            });
            if self.compress_program {
                debug!("Compressing hot binary");
                let compressed = Vec::new();
                let mut encoder = GzBuilder::new().write(compressed, Compression::default());
                encoder.write_all(hot).unwrap();
                *hot = encoder.finish().unwrap();
            }
            connection
                .execute_command(UploadFile {
                    filename: FixedLengthString::new(format!("{}.bin", base_file_name))?,
                    filetype: FixedLengthString::new("bin".to_string())?,
                    vendor: None,
                    data: hot.clone(),
                    target: None,
                    load_addr: 0x07800000,
                    linked_file,
                    after_upload: self.after_upload,
                    progress_callback: Some(Box::new(|progress| {
                        info!("Uploading hot: {:.2}%", progress)
                    })),
                })
                .await?;
        }

        Ok(())
    }
}
