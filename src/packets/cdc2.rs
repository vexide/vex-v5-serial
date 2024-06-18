use super::{DeviceBoundPacket, Encode, HostBoundPacket};

pub type Cdc2CommandPacket<const ID: u8, const EXT_ID: u8, P> =
    DeviceBoundPacket<Cdc2CommandPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandPayload<const ID: u8, P: Encode> {
    pub payload: P,
    pub crc: crc::Crc<u32>,
}
impl<const ID: u8, P: Encode> Encode for Cdc2CommandPayload<ID, P> {
    fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        let payload_bytes = self.payload.encode();
        let hash = self.crc.checksum(&payload_bytes);

        encoded.extend(payload_bytes);
        encoded.extend_from_slice(&hash.to_be_bytes());

        encoded
    }
}

pub type Cdc2ReplyPacket<const ID: u8, const EXT_ID: u8, P> =
    HostBoundPacket<Cdc2CommandReplyPayload<EXT_ID, P>, ID>;

pub struct Cdc2CommandReplyPayload<const ID: u8, P> {
    pub ack: u8,
    pub data: P,
    pub crc: crc::Crc<u32>,
}
