use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{encode::{Encode, EncodeError}, string::{DynamicVarLengthString, VarLengthString}};

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ControllerChannel {
    // NOTE: There's probably a secret third channel for matches, but that's not known.
    /// Used when controlling the robot outside of a competition match.
    Pit = 0x00,

    /// Used when wirelessly uploading or downloading data to/from the V5 Brain.
    ///
    /// Higher radio bandwidth for file transfer purposes.
    Download = 0x01,
}
impl Encode for ControllerChannel {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![*self as u8])
    }
}
pub type SwitchControllerChannelPacket = Cdc2CommandPacket<0x56, 0x10, SwitchcControllerChannelPayload>;
pub type SwitchControllerChannelReplyPacket = Cdc2ReplyPacket<0x56, 0x10, ()>;

#[derive(Debug, Clone)]
pub struct SwitchcControllerChannelPayload {
    /// PROS-cli sets this to 1.
    pub unknown: u8,
    pub channel: ControllerChannel,
}
impl Encode for SwitchcControllerChannelPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.unknown.to_le_bytes());
        encoded.extend(self.channel.encode()?);
        Ok(encoded)
    }
}

pub type UserFifoReadPacket = Cdc2CommandPacket<0x56, 0x27, UserFifoReadPayload>;
pub type UserFifoReadReplyPacket = Cdc2ReplyPacket<0x56, 0x27, UserFifoReadReplyPayload>;

pub struct UserFifoReadPayload {
    pub channel: ControllerChannel,
    pub length: u8,
}

pub struct UserFifoReadReplyPayload {
    pub unknown: u8,
    pub contents: DynamicVarLengthString,
}