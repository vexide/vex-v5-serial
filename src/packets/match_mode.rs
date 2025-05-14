use crate::encode::Encode;

use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Driver = 8,
    Auto = 10,
    Disabled = 11,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetMatchModePayload {
    pub match_mode: MatchMode,
    /// Time in seconds that should be displayed on the controller
    pub match_time: u32,
}
impl Encode for SetMatchModePayload {
    fn encode(&self) -> Result<Vec<u8>, crate::encode::EncodeError> {
        let mut encoded = Vec::new();
        encoded.push(self.match_mode as u8);
        encoded.extend(self.match_time.to_le_bytes());
        Ok(encoded)
    }
}

pub type SetMatchModePacket = Cdc2CommandPacket<0x58, 0xC1, SetMatchModePayload>;
pub type SetMatchModeReplyPacket = Cdc2ReplyPacket<0x58, 0xC1, ()>;
