use super::{
    cdc::cmds::{CON_CDC, USER_CDC},
    cdc2::{
        ecmds::{CON_COMP_CTRL, USER_READ},
        Cdc2CommandPacket, Cdc2ReplyPacket,
    },
};
use crate::{
    decode::{Decode, DecodeError, SizedDecode},
    encode::{Encode, EncodeError},
    string::FixedString,
};

pub type UserDataPacket = Cdc2CommandPacket<USER_CDC, USER_READ, UserDataPayload>;
pub type UserDataReplyPacket = Cdc2ReplyPacket<USER_CDC, USER_READ, UserDataReplyPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserDataPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Write (stdin) bytes.
    pub write: Option<FixedString<224>>,
}
impl Encode for UserDataPayload {
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserDataReplyPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Bytes read from stdout.
    pub data: Option<String>,
}
impl SizedDecode for UserDataReplyPayload {
    fn sized_decode(
        data: impl IntoIterator<Item = u8>,
        payload_size: u16,
    ) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let channel = u8::decode(&mut data)?;
        let data_len = payload_size.saturating_sub(5);

        let read = if data_len > 0 {
            Some({
                let mut utf8 = vec![];

                for _ in 0..data_len {
                    let byte = u8::decode(&mut data)?;

                    if byte == 0 {
                        break;
                    }

                    utf8.push(byte);
                }

                std::str::from_utf8(&utf8)?.to_string()
            })
        } else {
            None
        };

        Ok(Self {
            channel,
            data: read,
        })
    }
}

pub type CompetitionControlPacket =
    Cdc2CommandPacket<CON_CDC, CON_COMP_CTRL, CompetitionControlPayload>;
pub type CompetitionControlReplyPacket = Cdc2ReplyPacket<CON_CDC, CON_COMP_CTRL, ()>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Driver = 8,
    Auto = 10,
    Disabled = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionControlPayload {
    pub match_mode: MatchMode,
    /// Time in seconds that should be displayed on the controller
    pub match_time: u32,
}
impl Encode for CompetitionControlPayload {
    fn encode(&self) -> Result<Vec<u8>, crate::encode::EncodeError> {
        let mut encoded = Vec::new();
        encoded.push(self.match_mode as u8);
        encoded.extend(self.match_time.to_le_bytes());
        Ok(encoded)
    }
}
