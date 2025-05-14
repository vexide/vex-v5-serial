//! Global key-value store.

use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    encode::{Encode, EncodeError},
    string::FixedString,
};

pub type ReadKeyValuePacket = Cdc2CommandPacket<0x56, 0x2E, FixedString<31>>;
pub type ReadKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2E, FixedString<255>>;

pub type WriteKeyValuePacket = Cdc2CommandPacket<0x56, 0x2F, WriteKeyValuePayload>;
pub type WriteKeyValueReplyPacket = Cdc2ReplyPacket<0x56, 0x2F, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WriteKeyValuePayload {
    pub key: FixedString<31>,
    pub value: FixedString<255>,
}
impl Encode for WriteKeyValuePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend(self.key.as_ref().to_string().encode()?);
        encoded.extend(self.value.as_ref().to_string().encode()?);

        Ok(encoded)
    }
}
