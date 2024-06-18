use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

pub type ReadKeyValuePacket = Cdc2CommandPacket<0x56, 0x2e, String>;
pub type ReadKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2e, String>;

pub type WriteKeyValuePacket = Cdc2CommandPacket<0x56, 0x2f, WriteKeyValuePayload>;
pub type WriteKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2f, ()>;

pub struct WriteKeyValuePayload {
    pub key: String,
    pub value: String,
}
