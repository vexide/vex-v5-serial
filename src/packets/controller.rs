
use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    decode::{Decode, DecodeError, SizedDecode},
    encode::{Encode, EncodeError},
    string::{DynamicVarLengthString, VarLengthString},
};

pub type UserFifoPacket = Cdc2CommandPacket<86, 39, UserFifoPayload>;
pub type UserFifoReplyPacket = Cdc2ReplyPacket<86, 39, UserFifoReplyPayload>;

#[derive(Debug, Clone)]
pub struct UserFifoPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Write (stdin) bytes.
    pub write: Option<VarLengthString<224>>,
}
impl Encode for UserFifoPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.channel.to_le_bytes());
        if let Some(write) = &self.write {
            let encoded_write = write.encode()?;
            encoded.extend((encoded_write.len() as u8).to_le_bytes());
            encoded.extend(encoded_write);
        } else {
            encoded.extend([0]); // 0 write length
        }
        Ok(encoded)
    }
}

#[derive(Debug, Clone)]
pub struct UserFifoReplyPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Bytes read from stdout.
    pub data: Option<DynamicVarLengthString>,
}
impl SizedDecode for UserFifoReplyPayload {
    fn sized_decode(data: impl IntoIterator<Item = u8>, payload_size: u16) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let channel = u8::decode(&mut data)?;
        let data_len = payload_size - 5;
        
        let read = if data_len > 0 {
            Some(DynamicVarLengthString::decode_with_max_size(&mut data, (payload_size - 5) as usize)?)
        } else {
            None
        };

        Ok(Self {
            channel,
            data: read,
        })
    }
}
