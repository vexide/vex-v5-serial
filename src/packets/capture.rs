use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

pub type ScreenCapturePacket = Cdc2CommandPacket<0x56, 0x28, ()>;
pub type ScreenCaptureReplyPacket = Cdc2ReplyPacket<0x56, 0x28, ()>;