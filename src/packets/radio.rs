use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

pub struct RadioStatus {
    /// 0 = No controller, 4 = Controller connected (UNCONFIRMED)
    pub device: u8,
    /// From 0 to 100
    pub quality: u16,
    /// Always negative
    pub strength: i16,
    pub channel: i8,
    /// Latency between controller and brain (UNCONFIRMED)
    pub timeslot: i8,
}

pub type GetRadioStatusPacket = Cdc2CommandPacket<0x56, 0x26, ()>;
pub type GetRadioStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x26, RadioStatus>;
