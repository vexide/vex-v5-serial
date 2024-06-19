//! Global key-value store.

use super::{
    cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
    Encode, TerminatedFixedLengthString, VarLengthString,
};

pub type ReadKeyValuePacket = Cdc2CommandPacket<0x56, 0x2e, TerminatedFixedLengthString<31>>;
pub type ReadKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2e, VarLengthString<255>>;

pub type WriteKeyValuePacket = Cdc2CommandPacket<0x56, 0x2f, WriteKeyValuePayload>;
pub type WriteKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2f, ()>;

pub struct WriteKeyValuePayload {
    pub key: VarLengthString<31>,
    pub value: VarLengthString<255>,
}
impl Encode for WriteKeyValuePayload {
    fn encode(&self) -> Result<Vec<u8>, super::EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend(self.key.encode()?);
        encoded.extend(self.value.encode()?);

        Ok(encoded)
    }
}
