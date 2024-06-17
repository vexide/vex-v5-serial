use chrono::TimeZone;
use thiserror::Error;

use crate::{
    protocol::{FileTransferExit, FileTransferInit, FileTransferSetLink, FileTransferWrite},
    v5::{
        FileTransferComplete, FileTransferFunction, FileTransferOptions, FileTransferTarget,
        FileTransferType, FileTransferVID, V5FirmwareVersion,
    },
};

use super::{encode_string, j2000_timestamp, Command};

#[derive(Error, Debug)]
pub enum UploadFileError {
    #[error("Filename string encoding failed")]
    StringEncodeFailed(#[from] crate::commands::EncodeStringError),
    #[error("Response packet failed to decode")]
    ResponseDecodeFailed(#[from] crate::errors::DecodeError),
}

pub const COLD_START: u32 = 0x3800000;
const USER_PROGRAM_CHUNK_SIZE: u16 = 4096;

pub struct UploadFile {
    pub filename: String,
    pub filetype: FileTransferType,
    pub vendor: Option<FileTransferVID>,
    pub data: Vec<u8>,
    pub target: Option<FileTransferTarget>,
    pub load_addr: u32,
    pub linked_file: Option<Box<UploadFile>>,
    pub after_upload: FileTransferComplete,

    pub progress_callback: Option<Box<dyn FnMut(f32) + Send>>,
}
impl Command for UploadFile {
    type Error = UploadFileError;
    type Response = ();
    async fn execute(
        &mut self,
        device: &mut crate::devices::device::Device,
    ) -> Result<Self::Response, Self::Error> {
        let vendor = self.vendor.unwrap_or_default();
        let target = self.target.unwrap_or_default();

        let crc = crc::Crc::<u32>::new(&crate::VEX_CRC32).checksum(&self.data);

        let filename = encode_string::<24>(&self.filename)?;
        let length = filename.len();
        let mut string_bytes = [0; 24];
        string_bytes[..length].copy_from_slice(self.filename.as_bytes());

        let transfer_response = device
            .send_packet_request(FileTransferInit {
                function: FileTransferFunction::Upload,
                target,
                vid: vendor,
                options: FileTransferOptions::OVERWRITE,
                file_type: self.filetype,
                length: self.data.len() as _,
                addr: self.load_addr,
                crc,
                timestamp: j2000_timestamp(),
                version: V5FirmwareVersion {
                    major: 1,
                    minor: 0,
                    build: 0,
                    beta: 0,
                },
                name: string_bytes,
            })
            .await?;

        if let Some(linked_file) = &self.linked_file {
            let linked_filename = encode_string::<24>(linked_file.filename.as_str())?;
            let mut string_bytes = [0; 24];
            string_bytes[..linked_filename.len()].copy_from_slice(self.filename.as_bytes());
            device
                .send_packet_request(FileTransferSetLink(
                    string_bytes,
                    linked_file.vendor.unwrap_or_default(),
                    FileTransferOptions::OVERWRITE,
                ))
                .await?;
        }

        let max_chunk_size = if transfer_response.max_packet_size > 0
            && transfer_response.max_packet_size <= USER_PROGRAM_CHUNK_SIZE
        {
            transfer_response.max_packet_size
        } else {
            USER_PROGRAM_CHUNK_SIZE
        };

        let mut offset = 0;
        for chunk in self.data.chunks(max_chunk_size as _) {
            let progress = offset as f32 / self.data.len() as f32;
            if let Some(callback) = &mut self.progress_callback {
                callback(progress);
            }
            device
                .send_packet_request(FileTransferWrite(self.load_addr + offset, chunk))
                .await?;
            offset += chunk.len() as u32;
        }

        device
            .send_packet_request(FileTransferExit(self.after_upload))
            .await?;

        Ok(())
    }
}
