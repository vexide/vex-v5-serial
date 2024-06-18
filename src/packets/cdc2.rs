use super::{DeviceBoundPacket, HostBoundPacket};

pub type Cdc2CommandPacket<const ID: u8, const EXT_ID: u8, P> = DeviceBoundPacket<Cdc2CommandPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandPayload<const ID: u8, P> {
    pub payload: P,
    pub crc: crc::Algorithm<u32>,
}

pub type Cdc2ReplyPacket<const ID: u8, const EXT_ID: u8, P> = HostBoundPacket<Cdc2CommandReplyPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandReplyPayload<const ID: u8, P> {
    pub ack: u8,
    pub data: P,
    pub crc: crc::Algorithm<u32>,
}