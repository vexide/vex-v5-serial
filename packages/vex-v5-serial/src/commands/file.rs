use std::{
    io::Write,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use flate2::{Compression, GzBuilder};
use log::{debug, trace};

use crate::{Connection, ConnectionType};

use vex_cdc::{
    FixedString, VEX_CRC32, Version,
    cdc2::file::{
        ExtensionType, FileDataReadPacket, FileDataWritePacket, FileExitAction, FileInitOption,
        FileLinkPacket, FileMetadata, FileTransferExitPacket, FileTransferInitializePacket,
        FileTransferOperation, FileTransferTarget, FileVendor,
    },
};

use super::Command;

/// The epoch of the serial protocol's timestamps.
pub const J2000_EPOCH: u64 = 946684800;

pub fn j2000_timestamp() -> i32 {
    let unix_timestamp_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    (unix_timestamp_secs - J2000_EPOCH) as i32
}

pub const PROS_HOT_BIN_LOAD_ADDR: u32 = 0x7800000;
pub const USER_PROGRAM_LOAD_ADDR: u32 = 0x3800000;

pub async fn download_file<C: Connection>(
    connection: &mut C,
    file_name: FixedString<23>,
    size: u32,
    vendor: FileVendor,
    target: FileTransferTarget,
    address: u32,
    mut progress_callback: Option<impl FnMut(f32) + Send>,
) -> Result<Vec<u8>, C::Error> {
    let transfer_response = connection
        .handshake(
            FileTransferInitializePacket {
                operation: FileTransferOperation::Read,
                target,
                vendor,
                options: FileInitOption::None,
                file_size: size,
                write_file_crc: 0,
                load_address: address,
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
                file_name,
            },
            Duration::from_millis(500),
            5,
        )
        .await??;

    let max_chunk_size = connection
        .connection_type()
        .max_chunk_size(transfer_response.window_size);

    let mut data = Vec::with_capacity(transfer_response.file_size as usize);
    let mut offset = 0;
    loop {
        let read = connection
            .handshake(
                FileDataReadPacket {
                    address: address + offset,
                    size: max_chunk_size,
                },
                Duration::from_millis(500),
                5,
            )
            .await??;

        offset += read.data.len() as u32;
        let progress = (offset as f32 / transfer_response.file_size as f32) * 100.0;

        if let Some(callback) = &mut progress_callback {
            callback(progress);
        }

        if transfer_response.file_size <= offset {
            // Since data is returned in fixed-size chunks read from flash, VEXos will sometimes read
            // past the end of the file in the last chunk, returning whatever garbled nonsense happens
            // to be stored next in QSPI. This is a feature™️, and something we need to handle ourselves.
            let eof = read.data.len() - (offset - transfer_response.file_size) as usize;
            data.extend(&read.data[0..eof]);
            break; // we're done here
        } else {
            data.extend(read.data);
        }
    }

    Ok(data)
}

pub struct LinkedFile {
    pub file_name: FixedString<23>,
    pub vendor: FileVendor,
}

pub async fn upload_file<C: Connection + ?Sized>(
    connection: &mut C,
    file_name: FixedString<23>,
    metadata: FileMetadata,
    vendor: FileVendor,
    data: &[u8],
    target: FileTransferTarget,
    load_address: u32,
    linked_file: Option<LinkedFile>,
    after_upload: FileExitAction,
    mut progress_callback: Option<impl FnMut(f32) + Send>,
) -> Result<(), C::Error> {
    debug!("Uploading file: {}", file_name);
    let crc = VEX_CRC32.checksum(&data);

    let transfer_response = connection
        .handshake(
            FileTransferInitializePacket {
                operation: FileTransferOperation::Write,
                target,
                vendor,
                options: FileInitOption::Overwrite,
                file_size: data.len() as u32,
                load_address: load_address,
                write_file_crc: crc,
                metadata,
                file_name: file_name.clone(),
            },
            Duration::from_millis(500),
            5,
        )
        .await?;
    debug!("transfer init responded");
    let transfer_response = transfer_response?;

    if let Some(linked_file) = linked_file {
        connection
            .handshake(
                FileLinkPacket {
                    vendor: linked_file.vendor,
                    reserved: 0,
                    required_file: linked_file.file_name,
                },
                Duration::from_millis(500),
                5,
            )
            .await??;
    }

    let window_size = transfer_response.window_size;

    // The maximum packet size is 244 bytes for bluetooth
    let max_chunk_size = connection.connection_type().max_chunk_size(window_size);
    debug!("max_chunk_size: {}", max_chunk_size);

    let mut offset = 0;
    for chunk in data.chunks(max_chunk_size as _) {
        let chunk = if chunk.len() < max_chunk_size as _ && chunk.len() % 4 != 0 {
            let mut new_chunk = Vec::new();
            new_chunk.extend_from_slice(chunk);
            new_chunk.resize(chunk.len() + (4 - chunk.len() % 4), 0);
            new_chunk
        } else {
            chunk.to_vec()
        };
        trace!("sending chunk of size: {}", chunk.len());
        let progress = (offset as f32 / data.len() as f32) * 100.0;
        if let Some(callback) = &mut progress_callback {
            callback(progress);
        }

        let packet = FileDataWritePacket {
            address: (load_address + offset) as _,
            chunk_data: chunk.clone(),
        };

        // On bluetooth, we dont wait for the reply
        if connection.connection_type() == ConnectionType::Bluetooth {
            connection.send(packet).await?;
        } else {
            connection
                .handshake(packet, Duration::from_millis(500), 5)
                .await??;
        }

        offset += chunk.len() as u32;
    }
    if let Some(callback) = &mut progress_callback {
        callback(100.0);
    }

    connection
        .handshake(
            FileTransferExitPacket {
                action: after_upload,
            },
            Duration::from_millis(1000),
            5,
        )
        .await??;

    debug!("Successfully uploaded file: {}", file_name);
    Ok(())
}

#[derive(Debug)]
pub enum ProgramData {
    Monolith(Vec<u8>),
    HotCold {
        hot: Option<Vec<u8>>,
        cold: Option<Vec<u8>>,
    },
}

pub async fn upload_program<C: Connection + ?Sized>(
    connection: &mut C,
    slot: u8,
    name: &str,
    description: &str,
    program_type: &str,
    icon: &str,
    compress: bool,
    data: ProgramData,
    after_upload: FileExitAction,
    mut ini_callback: Option<impl FnMut(f32) + Send>,
    mut bin_callback: Option<impl FnMut(f32) + Send>,
    mut lib_callback: Option<impl FnMut(f32) + Send>,
) -> Result<(), C::Error> {
    let base_file_name = format!("slot_{}", slot);

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
        program_type,
        name,
        slot - 1,
        icon,
        description
    );

    upload_file(
        connection,
        FixedString::new(format!("{}.ini", base_file_name))?,
        FileMetadata {
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
        FileVendor::User,
        ini.as_bytes(),
        FileTransferTarget::Qspi,
        USER_PROGRAM_LOAD_ADDR,
        None,
        FileExitAction::DoNothing,
        ini_callback.as_mut().map(|cb| |p| cb(p)),
    )
    .await?;

    let program_bin_name = format!("{base_file_name}.bin");
    let program_lib_name = format!("{base_file_name}_lib.bin");

    let is_monolith = matches!(data, ProgramData::Monolith(_));
    let (program_data, library_data) = match data {
        ProgramData::HotCold { hot, cold } => (hot, cold),
        ProgramData::Monolith(data) => (Some(data), None),
    };

    if let Some(mut library_data) = library_data {
        debug!("Uploading cold library binary");

        // Compress the file to improve upload times
        // We don't need to change any other flags, the brain is smart enough to decompress it
        if compress {
            debug!("Compressing cold library binary");
            compress_data(&mut library_data);
            debug!("Compression complete");
        }

        upload_file(
            connection,
            FixedString::new(program_lib_name.clone())?,
            FileMetadata {
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
            FileVendor::User,
            &library_data,
            FileTransferTarget::Qspi,
            PROS_HOT_BIN_LOAD_ADDR,
            None,
            if is_monolith {
                after_upload
            } else {
                // we are still uploading, so the post-upload action should not yet be performed
                FileExitAction::DoNothing
            },
            lib_callback.as_mut().map(|cb| |p| cb(p)),
        )
        .await?;
    }

    if let Some(mut program_data) = program_data {
        debug!("Uploading program binary");

        if compress {
            debug!("Compressing program binary");
            compress_data(&mut program_data);
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

        upload_file(
            connection,
            FixedString::new(program_bin_name)?,
            FileMetadata {
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
            FileVendor::User,
            &program_data,
            FileTransferTarget::Qspi,
            USER_PROGRAM_LOAD_ADDR,
            linked_file,
            after_upload,
            bin_callback.as_mut().map(|cb| |p| cb(p)),
        )
        .await?;
    }

    Ok(())
}

/// Apply gzip compression to the given data
fn compress_data(data: &mut Vec<u8>) {
    let mut encoder = GzBuilder::new().write(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    *data = encoder.finish().unwrap();
}
