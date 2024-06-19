//! Global key-value store.

use super::{cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket}, TerminatedFixedLengthString};

pub type ReadKeyValuePacket = Cdc2CommandPacket<0x56, 0x2e, TerminatedFixedLengthString<31>>;
//TODO: Variable length string
pub type ReadKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2e, String>;

pub type WriteKeyValuePacket = Cdc2CommandPacket<0x56, 0x2f, WriteKeyValuePayload>;
pub type WriteKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2f, ()>;

//TODO: Variable length strings
pub struct WriteKeyValuePayload {
    pub key: String,
    pub value: String,
}
