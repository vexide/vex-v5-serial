use std::{
    io::Write,
    str::FromStr,
    time::{Duration, SystemTime},
};

use flate2::{Compression, GzBuilder};
use log::{debug, trace};

use crate::{Connection, ConnectionType};

use vex_cdc::{
    cdc2::file::{
        ExtensionType, FileDataReadPacket, FileDataReadPayload, FileDataReadReplyPacket,
        FileDataWritePacket, FileDataWritePayload, FileDataWriteReplyPacket, FileExitAction,
        FileInitOption, FileLinkPacket, FileLinkPayload, FileLinkReplyPacket, FileMetadata,
        FileTransferExitPacket, FileTransferExitReplyPacket, FileTransferInitializePacket,
        FileTransferInitializePayload, FileTransferInitializeReplyPacket, FileTransferOperation,
        FileTransferTarget, FileVendor,
    },
    FixedString, Version, VEX_CRC32,
};

use super::Command;

/// The epoch of the serial protocol's timestamps.
pub const J2000_EPOCH: u32 = 946684800;

pub fn j2000_timestamp() -> i32 {
    (SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        - J2000_EPOCH as u128) as i32
}

pub const PROS_HOT_BIN_LOAD_ADDR: u32 = 0x7800000;
pub const USER_PROGRAM_LOAD_ADDR: u32 = 0x3800000;

pub struct DownloadFile {
    pub file_name: FixedString<23>,
    pub size: u32,
    pub vendor: FileVendor,
    pub target: FileTransferTarget,
    pub address: u32,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for DownloadFile {
    type Output = Vec<u8>;

    async fn execute<C: Connection + ?Sized>(
        mut self,
        connection: &mut C,
    ) -> Result<Self::Output, C::Error> {
        let transfer_response = connection
            .handshake::<FileTransferInitializeReplyPacket>(
                Duration::from_millis(500),
                5,
                FileTransferInitializePacket::new(FileTransferInitializePayload {
                    operation: FileTransferOperation::Read,
                    target: self.target,
                    vendor: self.vendor,
                    options: FileInitOption::None,
                    file_size: self.size,
                    write_file_crc: 0,
                    load_address: self.address,
                    metadata: FileMetadata {
                        extension: FixedString::from_str("ini").unwrap(),
                        extension_type: ExtensionType::EncryptedBinary,
                        timestamp: 0,
                        version: Version {
                            major: 1,
                            minor: 0,
                            build: 0,
                            beta: 0,
                        },
                    },
                    file_name: self.file_name,
                }),
            )
            .await?;
        let transfer_response = transfer_response.payload?;

        let max_chunk_size = connection.connection_type().max_chunk_size(transfer_response.window_size);

        let mut data = Vec::with_capacity(transfer_response.file_size as usize);
        let mut offset = 0;
        loop {
            let read = connection
                .handshake::<FileDataReadReplyPacket>(
                    Duration::from_millis(500),
                    5,
                    FileDataReadPacket::new(FileDataReadPayload {
                        address: self.address + offset,
                        size: max_chunk_size,
                    }),
                )
                .await?;

            let (_, chunk_data) = read.payload.unwrap()?;
            offset += chunk_data.len() as u32;
            let progress = (offset as f32 / transfer_response.file_size as f32) * 100.0;

            if let Some(callback) = &mut self.progress_callback {
                callback(progress);
            }

            if transfer_response.file_size <= offset {
                // Since data is returned in fixed-size chunks read from flash, VEXos will sometimes read
                // past the end of the file in the last chunk, returning whatever garbled nonsense happens
                // to be stored next in QSPI. This is a feature™️, and something we need to handle ourselves.
                let eof = chunk_data.len() - (offset - transfer_response.file_size) as usize;
                data.extend(&chunk_data[0..eof]);
                break; // we're done here
            } else {
                data.extend(chunk_data);
            }
        }

        Ok(data)
    }
}

pub struct LinkedFile {
    pub file_name: FixedString<23>,
    pub vendor: FileVendor,
}

pub struct UploadFile<'a> {
    pub file_name: FixedString<23>,
    pub metadata: FileMetadata,
    pub vendor: FileVendor,
    pub data: &'a [u8],
    pub target: FileTransferTarget,
    pub load_address: u32,
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
        debug!("Uploading file: {}", self.file_name);
        let crc = VEX_CRC32.checksum(&self.data);

        let transfer_response = connection
            .handshake::<FileTransferInitializeReplyPacket>(
                Duration::from_millis(500),
                5,
                FileTransferInitializePacket::new(FileTransferInitializePayload {
                    operation: FileTransferOperation::Write,
                    target: self.target,
                    vendor: self.vendor,
                    options: FileInitOption::Overwrite,
                    file_size: self.data.len() as u32,
                    load_address: self.load_address,
                    write_file_crc: crc,
                    metadata: self.metadata,
                    file_name: self.file_name.clone(),
                }),
            )
            .await?;
        debug!("transfer init responded");
        let transfer_response = transfer_response.payload?;

        if let Some(linked_file) = self.linked_file {
            connection
                .handshake::<FileLinkReplyPacket>(
                    Duration::from_millis(500),
                    5,
                    FileLinkPacket::new(FileLinkPayload {
                        vendor: linked_file.vendor,
                        reserved: 0,
                        required_file: linked_file.file_name,
                    }),
                )
                .await?
                .payload?;
        }

        let window_size = transfer_response.window_size;

        // The maximum packet size is 244 bytes for bluetooth
        let max_chunk_size = connection.connection_type().max_chunk_size(window_size);
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

            let packet = FileDataWritePacket::new(FileDataWritePayload {
                address: (self.load_address + offset) as _,
                chunk_data: chunk.clone(),
            });

            // On bluetooth, we dont wait for the reply
            if connection.connection_type() == ConnectionType::Bluetooth {
                connection.send(packet).await?;
            } else {
                connection
                    .handshake::<FileDataWriteReplyPacket>(Duration::from_millis(500), 5, packet)
                    .await?
                    .payload?;
            }

            offset += chunk.len() as u32;
        }
        if let Some(callback) = &mut self.progress_callback {
            callback(100.0);
        }

        connection
            .handshake::<FileTransferExitReplyPacket>(
                Duration::from_millis(1000),
                5,
                FileTransferExitPacket::new(self.after_upload),
            )
            .await?
            .payload?;

        debug!("Successfully uploaded file: {}", self.file_name);
        Ok(())
    }
}

#[derive(Debug)]
pub enum ProgramData {
    Monolith(Vec<u8>),
    HotCold {
        hot: Option<Vec<u8>>,
        cold: Option<Vec<u8>>,
    },
}

pub struct UploadProgram<'a> {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub program_type: String,
    /// 0-indexed slot
    pub slot: u8,
    pub compress: bool,
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

        debug!("Uploading program ini file");

        let ini = format!(
            "[project]
ide={}
[program]
name={}
slot={}
icon={}
iconalt=
description={}",
            self.program_type,
            self.name,
            self.slot - 1,
            self.icon,
            self.program_type
        );

        connection
            .execute_command(UploadFile {
                file_name: FixedString::new(format!("{}.ini", base_file_name))?,
                metadata: FileMetadata {
                    extension: unsafe { FixedString::new_unchecked("ini") },
                    extension_type: ExtensionType::default(),
                    timestamp: j2000_timestamp(),
                    version: Version {
                        major: 1,
                        minor: 0,
                        build: 0,
                        beta: 0,
                    },
                },
                vendor: FileVendor::User,
                data: ini.as_bytes(),
                target: FileTransferTarget::Qspi,
                load_address: USER_PROGRAM_LOAD_ADDR,
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
            debug!("Uploading cold library binary");

            // Compress the file to improve upload times
            // We don't need to change any other flags, the brain is smart enough to decompress it
            if self.compress {
                debug!("Compressing cold library binary");
                compress(&mut library_data);
                debug!("Compression complete");
            }

            connection
                .execute_command(UploadFile {
                    file_name: FixedString::new(program_lib_name.clone())?,
                    metadata: FileMetadata {
                        extension: unsafe { FixedString::new_unchecked("bin") },
                        extension_type: ExtensionType::default(),
                        timestamp: j2000_timestamp(),
                        version: Version {
                            major: 1,
                            minor: 0,
                            build: 0,
                            beta: 0,
                        },
                    },
                    vendor: FileVendor::User,
                    data: &library_data,
                    target: FileTransferTarget::Qspi,
                    load_address: PROS_HOT_BIN_LOAD_ADDR,
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
            debug!("Uploading program binary");

            if self.compress {
                debug!("Compressing program binary");
                compress(&mut program_data);
                debug!("Compression complete");
            }

            // Only ask the brain to link to a library if the program expects it.
            // Monolith programs don't have libraries.
            let linked_file = if is_monolith {
                None
            } else {
                debug!("Program will be linked to cold library: {program_lib_name:?}");
                Some(LinkedFile {
                    file_name: FixedString::new(program_lib_name)?,
                    vendor: FileVendor::User,
                })
            };

            connection
                .execute_command(UploadFile {
                    file_name: FixedString::new(program_bin_name)?,
                    metadata: FileMetadata {
                        extension: unsafe { FixedString::new_unchecked("bin") },
                        extension_type: ExtensionType::default(),
                        timestamp: j2000_timestamp(),
                        version: Version {
                            major: 1,
                            minor: 0,
                            build: 0,
                            beta: 0,
                        },
                    },
                    vendor: FileVendor::User,
                    data: &program_data,
                    target: FileTransferTarget::Qspi,
                    load_address: USER_PROGRAM_LOAD_ADDR,
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
