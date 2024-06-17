//! Implements commands that deal directly with the V5 system

use super::Command;

#[derive(Copy, Clone, Debug)]
pub struct GetSystemVersion();

impl Command for GetSystemVersion {
    type Response = V5SystemVersion;

    fn encode_request(self) -> Result<(u8, Vec<u8>), crate::errors::DecodeError> {
        // Just encode an empty command with id 0xA4
        Ok((0xA4, vec![]))
    }

    fn decode_response(
        command_id: u8,
        data: Vec<u8>,
    ) -> Result<Self::Response, crate::errors::DecodeError> {
        // Make sure we are recieving the right command
        // Ensure that it is a system info packet
        if command_id != 0xA4 {
            return Err(crate::errors::DecodeError::ExpectedExtended);
        }

        // Alias to make code shorter
        let v = data;

        // Get and return the V5SystemVersion
        Ok(V5SystemVersion {
            system_version: (v[0], v[1], v[2], v[3], v[4]),
            product_type: crate::v5::VexProductType::try_from((v[5], v[6]))?,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct V5SystemVersion {
    pub system_version: (u8, u8, u8, u8, u8),
    pub product_type: crate::v5::VexProductType,
}
