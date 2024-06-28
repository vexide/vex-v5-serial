use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

pub type ScreenCapturePacket = Cdc2CommandPacket<86, 40, ()>;
pub type ScreenCaptureReplyPacket = Cdc2ReplyPacket<86, 40, ()>;
