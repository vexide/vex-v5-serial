//! Global key-value store.

use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    encode::{Encode, EncodeError},
    string::{FixedLengthString, VarLengthString},
};

pub type ReadKeyValuePacket = Cdc2CommandPacket<86, 46, FixedLengthString<31>>;
pub type ReadKeyValueReplyPacket = Cdc2ReplyPacket<86, 46, VarLengthString<255>>;

pub type WriteKeyValuePacket = Cdc2CommandPacket<86, 47, WriteKeyValuePayload>;
pub type WriteKeyValueReplyPacket = Cdc2ReplyPacket<86, 47, ()>;

pub struct WriteKeyValuePayload {
    pub key: VarLengthString<31>,
    pub value: VarLengthString<255>,
}
impl Encode for WriteKeyValuePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend(self.key.encode()?);
        encoded.extend(self.value.encode()?);

        Ok(encoded)
    }
}
