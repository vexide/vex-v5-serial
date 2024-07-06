use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    string::VarLengthString,
};

pub type UserFifoPacket = Cdc2CommandPacket<86, 39, UserFifoPayload>;
pub type UserFifoReplyPacket = Cdc2ReplyPacket<86, 39, UserFifoReplyPayload>;

#[derive(Debug, Clone)]
pub struct UserFifoPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Number of bytes from stdin that should be read.
    pub read_length: u8,

    /// Write (stdin) bytes.
    pub write: Option<VarLengthString<224>>,
}
impl Encode for UserFifoPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.channel.to_le_bytes());
        encoded.extend(self.read_length.to_le_bytes());
        if let Some(write) = &self.write {
            encoded.extend(write.encode()?);
        }
        Ok(encoded)
    }
}

#[derive(Debug, Clone)]
pub struct UserFifoReplyPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Bytes read from stdout.
    pub data: VarLengthString<64>,
}
impl Decode for UserFifoReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let channel = u8::decode(&mut data)?;
        let read = VarLengthString::<64>::decode(&mut data)?;
        Ok(Self {
            channel,
            data: read,
        })
    }
}
