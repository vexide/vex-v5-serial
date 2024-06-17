//! Implements two structures: One for reading key-value entries on the brain, and one for writing key-value entries to the brain.

use super::Command;

/// Reads in a key-value entry from the brain.
///
/// # Members
///
/// * `0` - A string slice that contains the key to read from the brain
///
/// # Examples
///
/// ```rust
///
/// use vexv5_serial::commands::KVRead;
///
/// // Create a KVRead instance that will generate a command to read the teamnumber key
/// let kv = KVRead("teamnumber");
///
/// // The only other really useful key is the robotname key
/// let kv = KVRead("robotname");
///
/// ```
#[derive(Copy, Clone)]
pub struct KVRead<'a>(pub &'a str);

impl<'a> Command for KVRead<'a> {
    type Response = String;

    /// Encodes a request for the value of a key-value store.
    /// The &str in the struct body is used as the key
    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // The payload is just the key, but zero terminated
        let mut payload = self.0.as_bytes().to_vec();
        payload.push(0);

        // Encode an extended command of value 0x2e
        super::Extended(0x2e, &payload).encode_request()
    }

    /// Returns the String value of the key requested.
    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Read in the extended packet
        let packet = super::Extended::decode_response(command_id, data)?;

        // If the command id is wrong, then error
        if packet.0 != 0x2e {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x2e, packet.0));
        }

        // The payload of the packet should just be the value of the kv store
        // minus the null-terminator
        // Suffix here is always &[0] so it will always return Some. We can just unwrap
        Ok(String::from_utf8(
            packet.1.strip_suffix(&[0]).unwrap().to_vec(),
        )?)
    }
}

/// Writes a key-value entry to the brain
///
/// # Members
///
/// * `0` - A string slice that contains the key to write to on the brain
/// * `1` - A string slice that contains the value to write to the key-value store
///
/// # Examples
///
/// ```rust
///
/// use vexv5_serial::commands::KVWrite;
///
/// // Create a KVWrite instance that will write to the teamnumber key
/// let kv = KVWrite("teamnumber", "ABCD");
///
/// // We can also do the same with the robotname key, which is the
/// // only other key that is useful to most users
/// let kv = KVWrite("robotname", "robo");
///
/// ```
#[derive(Copy, Clone)]
pub struct KVWrite<'a>(pub &'a str, pub &'a str);

impl<'a> Command for KVWrite<'a> {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Convert the value to an array of bytes
        let value = self.1.as_bytes();

        // Certain keys have a maximum size
        let packet_length = {
            usize::min(self.1.len(), {
                if self.0 == "teamnumber" {
                    7
                } else if self.0 == "robotname" {
                    16
                } else {
                    254
                }
            })
        };

        // Trim the value to the maximum size and convert to a vec so we can push the null-terminator
        let mut value = value[..packet_length].to_vec();
        value.push(0); // Null terminator

        // Likewise convert the key and add a null-terminator
        let mut key = self.0.as_bytes().to_vec();
        key.push(00);

        // The payload is just b"{key}{value}"
        // We will use key as the payload
        key.extend(value);

        // Send the extended command
        super::Extended(0x2f, &key).encode_request()
    }

    /// This returns `()`, and if a package is malformed or not recieved it may return an error.
    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Decode as an extended packet
        let packet = super::Extended::decode_response(command_id, data)?;

        // If the command id is wrong, then error
        if packet.0 != 0x2f {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x2e, packet.0));
        }

        Ok(())
    }
}
