use serde::{Deserialize, Serialize};

use crate::{
    checks::VexExtPacketChecks,
    v5::{
        FileMetadataByName, FileTransferComplete, FileTransferFunction, FileTransferOptions,
        FileTransferTarget, FileTransferType, FileTransferVID, V5FirmwareVersion,
    },
};

use super::Packet;

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
    pub program: Program,
    pub project: Project,
}

/// Initializes a file transfer between the brain and host
#[derive(Copy, Clone)]
pub struct FileTransferInit {
    pub function: FileTransferFunction,
    pub target: FileTransferTarget,
    pub vid: FileTransferVID,
    pub options: FileTransferOptions,
    pub file_type: FileTransferType,
    pub length: u32,
    pub addr: u32,
    pub crc: u32,
    pub timestamp: u32,
    pub version: V5FirmwareVersion,
    pub name: [u8; 24],
}

impl Packet for FileTransferInit {
    type Response = FileTransferInitResponse;

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Create the empty payload
        let mut payload = Vec::<u8>::new();

        // Load the function, target, vid, and options
        payload.extend([
            self.function as u8,
            self.target as u8,
            self.vid.to_u8(),
            self.options.bits(),
        ]);

        // Add the length
        payload.extend(self.length.to_le_bytes());

        // Add the addr
        payload.extend(self.addr.to_le_bytes());

        // Add the crc
        payload.extend(self.crc.to_le_bytes());

        // Add the type
        payload.extend(self.file_type.to_bytes());

        // Add the timestamp
        payload.extend(self.timestamp.to_le_bytes());

        // Add the version
        payload.extend(self.version.into_le_bytes());

        // Add the file name to the payload
        payload.extend(self.name);

        // Encode an extended command with id 0x11
        super::Extended(0x11, &payload).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x11
        if payload.0 != 0x11 {
            return Err(crate::errors::DecodeError::ExpectedPacket(0x11, payload.0));
        }

        // Get the max_packet_size (bytes 0..1)
        // We can unwrap the try_into because we know that get will return 2 bytes
        let max_packet_size = u16::from_le_bytes(
            payload
                .1
                .get(0..2)
                .ok_or(crate::errors::DecodeError::PacketLengthError)?
                .try_into()
                .unwrap(),
        );

        // Get the file_size (bytes 2..3)
        let file_size = u32::from_le_bytes(
            payload
                .1
                .get(2..6)
                .ok_or(crate::errors::DecodeError::PacketLengthError)?
                .try_into()
                .unwrap(),
        );

        // Get the crc (bytes 4..8)
        let crc = u32::from_le_bytes(
            payload
                .1
                .get(6..10)
                .ok_or(crate::errors::DecodeError::PacketLengthError)?
                .try_into()
                .unwrap(),
        );

        // Return the result
        Ok(FileTransferInitResponse {
            max_packet_size,
            file_size,
            crc,
        })
    }
}

#[derive(Copy, Clone)]
pub struct FileTransferInitResponse {
    pub max_packet_size: u16,
    pub file_size: u32,
    pub crc: u32,
}

/// Exit a file transfer between the brain and host
///
/// # Members
///
/// * `0` - The action to complete when the transfer is finished
#[derive(Copy, Clone)]
pub struct FileTransferExit(pub FileTransferComplete);

impl Packet for FileTransferExit {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Create the empty payload and
        // Add the file transfer complete byte
        let payload = vec![self.0 as u8];

        // Encode an extended command with id 0x12
        super::Extended(0x12, &payload).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x12
        if payload.0 != 0x12 {
            return Err(crate::errors::DecodeError::ExpectedPacket(0x12, payload.0));
        }

        // Do nothing
        Ok(())
    }
}

/// Sets the linked file for the current transfer
///
/// # Members
///
/// * `0` - The linked file name
/// * `1` - The file VID
/// * `2` - The file options
#[derive(Copy, Clone)]
pub struct FileTransferSetLink(pub [u8; 24], pub FileTransferVID, pub FileTransferOptions);

impl Packet for FileTransferSetLink {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Create the packet
        let mut packet = Vec::<u8>::new();

        // Add the vid
        packet.push(self.1.to_u8());

        // Add the options
        packet.push(self.2.bits());

        // Add the name
        packet.extend(self.0);

        super::Extended(0x15, &packet).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x15
        if payload.0 != 0x15 {
            return Err(crate::errors::DecodeError::ExpectedPacket(0x15, payload.0));
        }

        Ok(())
    }
}

/// Read data from a file transfer
///
/// # Members
///
/// * `0` - The address to read data from
/// * `1` - The number of bytes to read, will be padded to 4 bytes
#[derive(Copy, Clone)]
pub struct FileTransferRead(pub u32, pub u16);

impl Packet for FileTransferRead {
    type Response = Vec<u8>;

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Pad nbytes to a 4 byte barrier
        let nbytes = if self.1 % 4 == 0 {
            self.1
        } else {
            self.1 + 4 - (self.1 % 4)
        };

        // Create the payload
        let mut payload = Vec::<u8>::new();

        // Add the address
        payload.extend(self.0.to_le_bytes());

        // Add the data length
        payload.extend(nbytes.to_le_bytes());

        // Return the encoded extended packet with id 0x14
        super::Extended(0x14, &payload).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Read the extended command
        let payload = super::Extended::decode_extended(
            command_id,
            data,
            VexExtPacketChecks::LENGTH | VexExtPacketChecks::CRC,
        )?;

        // Ensure that it is a response to 0x14
        if payload.0 != 0x14 {
            return Err(crate::errors::DecodeError::ExpectedPacket(0x14, payload.0));
        }

        // Return the data
        Ok(payload.1)
    }
}

/// Write data to a file transfer
///
/// # Members
///
/// * `0` - The address to write at
/// * `1` - The data to write
#[derive(Copy, Clone)]
pub struct FileTransferWrite<'a>(pub u32, pub &'a [u8]);

impl<'a> Packet for FileTransferWrite<'a> {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Create the payload vec
        let mut packet = Vec::<u8>::new();

        // Pad the payload to 4 bytes
        let mut payload = self.1.to_vec();
        payload.resize(
            if payload.len() % 4 == 0 {
                payload.len()
            } else {
                payload.len() + 4 - (payload.len() % 4)
            },
            0,
        );

        // Add the address to the packet
        packet.extend(self.0.to_le_bytes());

        // Add the payload to the packet
        packet.extend(payload);

        // Return the parsed extended packet with id 0x13
        super::Extended(0x13, &packet).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Read the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x13
        if payload.0 != 0x13 {
            return Err(crate::errors::DecodeError::ExpectedPacket(0x13, payload.0));
        }

        // Return Ok
        Ok(())
    }
}

/// Gets file metadata by file name
///
/// # Members
///
/// * `0` - The name of the file
/// * `1` - The VID of the file
/// * `2` - The file transfer options -- Use NONE
///
#[derive(Copy, Clone, Debug)]
pub struct GetFileMetadataByName<'a>(
    pub &'a [u8; 24],
    pub FileTransferVID,
    pub FileTransferOptions,
);

impl<'a> Packet for GetFileMetadataByName<'a> {
    type Response = FileMetadataByName;

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Create the payload with the vid and optione
        let mut payload = vec![self.1.to_u8(), self.2.bits()];

        // Add the file name
        payload.extend(self.0);

        // Return the extended command with id 0x19
        super::Extended(0x19, &payload).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Read the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x19
        if payload.0 != 0x19 {
            return Err(crate::errors::DecodeError::ExpectedPacket(0x19, payload.0));
        }

        // Ensure that the payload size is at least 49 bytes
        if payload.1.len() < 49 {
            return Err(crate::errors::DecodeError::PacketLengthError);
        }

        // Parse in the data
        let result = FileMetadataByName {
            linked_vid: FileTransferVID::from_u8(payload.1[0]),
            length: u32::from_le_bytes(payload.1[1..5].try_into().unwrap()),
            addr: u32::from_le_bytes(payload.1[5..9].try_into().unwrap()),
            crc: u32::from_le_bytes(payload.1[9..13].try_into().unwrap()),
            file_type: FileTransferType::from_bytes(payload.1[13..17].try_into().unwrap()),
            timestamp: u32::from_le_bytes(payload.1[17..21].try_into().unwrap()),
            version: u32::from_le_bytes(payload.1[21..25].try_into().unwrap()),
            linked_filename: payload.1[25..49].try_into().unwrap(),
        };

        // Return the data
        Ok(result)
    }
}
