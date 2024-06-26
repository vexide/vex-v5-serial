use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    array::Array,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
};

pub struct Log {
    /// (RESEARCH NEEDED)
    pub code: u8,

    /// The subtype under the description (RESEARCH NEEDED)
    pub log_type: u8,

    /// The type of the log message (RESEARCH NEEDED)
    pub description: u8,

    /// (RESEARCH NEEDED)
    pub spare: u8,

    /// How long (in milliseconds) after the brain powered on
    pub time: u16,
}
impl Decode for Log {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let code = u8::decode(&mut data)?;
        let log_type = u8::decode(&mut data)?;
        let description = u8::decode(&mut data)?;
        let spare = u8::decode(&mut data)?;
        let time = u16::decode(&mut data)?;
        Ok(Self {
            code,
            log_type,
            description,
            spare,
            time,
        })
    }
}

pub type GetLogCountPacket = Cdc2CommandPacket<86, 36, ()>;
pub type GetLogCountReplyPacket = Cdc2ReplyPacket<86, 36, GetLogCountReplyPayload>;

pub struct GetLogCountReplyPayload {
    pub unknown: u8,
    pub count: u32,
}
impl Decode for GetLogCountReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let unknown = u8::decode(&mut data)?;
        let count = u32::decode(&mut data)?;
        Ok(Self { unknown, count })
    }
}

/// For example: If the brain has 26 logs, from A to Z. With offset 5 and count 5, it returns [V, W, X, Y, Z]. With offset 10 and count 5, it returns [Q, R, S, T, U].
pub type ReadLogPagePacket = Cdc2CommandPacket<86, 37, ReadLogPagePayload>;
pub type ReadLogPageReplyPacket = Cdc2ReplyPacket<86, 37, ReadLogPageReplyPayload>;

#[derive(Debug, Clone, Copy)]
pub struct ReadLogPagePayload {
    pub offset: u32,
    pub count: u32,
}
impl Encode for ReadLogPagePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.offset.to_le_bytes());
        encoded.extend(self.count.to_le_bytes());
        Ok(encoded)
    }
}

pub struct ReadLogPageReplyPayload {
    /// The offset number used in this packet.
    pub offset: u32,
    /// Number of elements in the following array.
    pub count: u32,
    pub entries: Array<Log>,
}
impl Decode for ReadLogPageReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let offset = u32::decode(&mut data)?;
        let count = u32::decode(&mut data)?;
        let entries = Array::decode_with_len(&mut data, count as _)?;
        Ok(Self {
            offset,
            count,
            entries,
        })
    }
}
