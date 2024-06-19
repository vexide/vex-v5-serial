use super::{
    cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
    Encode,
};

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
    fn encode(&self) -> Result<Vec<u8>, super::EncodeError> {
        Ok(vec![*self as u8])
    }
}
pub type SwitchControllerChannelPacket = Cdc2CommandPacket<0x56, 0x10, ControllerChannel>;
pub type SwitchControllerChannelReplyPacket = Cdc2ReplyPacket<0x56, 0x10, ()>;

// pub type UserFifoPacket = Cdc2CommandPacket<0x56, 0x27, ?>;
// pub type UserFifoResponsePacket = Cdc2ReplyPacket<0x56, 0x27, ?>;
