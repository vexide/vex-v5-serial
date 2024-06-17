//! Implements a command for setting the controller's channel

use super::Packet;
use crate::v5::V5ControllerChannel;

/// Switches the controller's channel
///
/// # Members
///
/// * `0` - The controller channel to switch to
///
/// # Examples
///
/// ```rust
///
/// use vexv5_serial::commands::SwitchChannel;
///
/// // Create a SwitchChannel instance that will switch to the download channel
/// let kv = SwitchChannel(V5ControllerChannel::Download);
///
/// // Create a SwitchChannel instance that will switch to the pit channel
/// let kv = SwitchChannel(V5ControllerChannel::Pit);
///
/// ```
#[derive(Copy, Clone)]
pub struct SwitchChannel(pub V5ControllerChannel);

impl Packet for SwitchChannel {
    type Response = ();

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Create the packet contents containing the Controller Channel to switch to
        // encoded as a u8 and return the extended packet with id 0x10
        super::Extended(0x10, &[self.0 as u8]).encode_request()
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Decode the extended command
        let payload = super::Extended::decode_response(command_id, data)?;

        // Ensure that it is a response to 0x10
        if payload.0 != 0x10 {
            return Err(crate::errors::DecodeError::ExpectedCommand(0x10, payload.0));
        }

        // Nothing needs to be returned
        Ok(())
    }
}
