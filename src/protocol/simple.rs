//! Implements a structure for encoding and decoding simple commands.

use super::Command;

/// The structure base of all Simple commands
/// Depended upon by all simple commands
///
/// # Members
///
/// * `0` - The simple command id of the command to send. 0x56 is Extended command
/// * `1` - The payload of the simple command being sent
#[derive(Copy, Clone)]
pub struct Simple<'a>(pub u8, pub &'a [u8]);

impl<'a> Command for Simple<'a> {
    type Response = SimpleResponse;

    fn encode_request(self) -> Vec<u8> {
        // Create the simple packet with magic number and command type
        let mut packet = vec![0xc9, 0x36, 0xb8, 0x47, self.0];

        // And just append the payload
        packet.extend(self.1);

        // Return the finished simple packet
        packet
    }

    fn decode_stream<T: std::io::Read>(
        stream: &mut T,
        timeout: std::time::Duration,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // We need to wait to recieve the header of a packet.
        // The header should be the bytes [0xAA, 0x55]

        // This header needs to be recieved within the timeout.
        // If it is not recieved within the timeout, then we need to return an error.
        // Begin the countdown now:
        let countdown = std::time::SystemTime::now() + timeout;

        // Create a buffer for the header bytes
        // This is configurable just in case vex changes the header bytes on us.
        let expected_header: [u8; 2] = [0xAA, 0x55];
        let mut header_index = 0; // This represents what index in the header we will be checking next.

        // The way this works is we recieve a byte from the device.
        // If it matches the current byte (expected_header[header_index]), then we increment the header_index.
        // If the header_index is equal to the length of the header, then we know we have recieved the header.
        // If the header_index is not equal to the length of the header, then we need to keep recieving bytes until we have recieved the header.
        // If an unexpected byte is recieved, reset header_index to zero.
        while header_index < expected_header.len() {
            // If the timeout has elapsed, then we need to return an error.
            // We need to do this first just in case we actually do recieve the header
            // before the timeout has elapsed.
            if countdown < std::time::SystemTime::now() {
                return Err(crate::errors::DecodeError::HeaderTimeout);
            }

            // Recieve a single bytes
            let mut b: [u8; 1] = [0];
            match stream.read_exact(&mut b) {
                // Do some match magic to convert the error types
                Ok(v) => Ok(v),
                Err(e) => Err(crate::errors::DecodeError::IoError(e)),
            }?;
            let b = b[0];

            if b == expected_header[header_index] {
                header_index += 1;
            } else {
                header_index = 0;
            }
        }

        // Now that we know we have recieved the header, we need to recieve the rest of the packet.

        // First create a vector containing the entirety of the recieved packet
        let mut packet: Vec<u8> = Vec::from(expected_header);

        // Read int he next two bytes
        let mut b: [u8; 2] = [0; 2];
        match stream.read_exact(&mut b) {
            // Do some match magic to convert the error types
            Ok(v) => Ok(v),
            Err(e) => Err(crate::errors::DecodeError::IoError(e)),
        }?;
        packet.extend_from_slice(&b);

        // Get the command byte and the length byte of the packet
        let command = b[0];

        // We may need to modify the length of the packet if it is an extended command
        // Extended commands use a u16 instead of a u8 for the length.
        let length = if 0x56 == command && b[1] & 0x80 == 0x80 {
            // Read the lower bytes
            let mut bl: [u8; 1] = [0];
            match stream.read_exact(&mut bl) {
                // Do some match magic to convert the error types
                Ok(v) => Ok(v),
                Err(e) => Err(crate::errors::DecodeError::IoError(e)),
            }?;
            packet.push(bl[0]);

            (((b[1] & 0x7f) as u16) << 8) | (bl[0] as u16)
        } else {
            b[1] as u16
        };

        // Read the rest of the payload
        let mut payload: Vec<u8> = vec![0; length as usize];
        // DO NOT CHANGE THIS TO READ. read_exact is required to suppress
        // CRC errors and missing data.
        match stream.read_exact(&mut payload) {
            // Do some match magic to convert the error types
            Ok(v) => Ok(v),
            Err(e) => Err(crate::errors::DecodeError::IoError(e)),
        }?;
        packet.extend(&payload);

        Ok(SimpleResponse(command, payload, packet))
    }
}

/// The response to all simple commands
///
/// # Members
///
/// * `0` - The simple command ID
/// * `1` - The payload of the simple command
/// * `2` - The entire response, including header, payload, and more. Used by Extended command to verify CRC.
pub struct SimpleResponse(pub u8, pub Vec<u8>, pub Vec<u8>);
