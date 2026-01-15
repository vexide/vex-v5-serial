//! Controller packets.

use alloc::{
    string::{String, ToString},
    vec,
};

use crate::{
    Decode, DecodeError, Encode, FixedString,
    cdc::cmds::{CON_CDC, USER_CDC},
    cdc2::{
        Cdc2CommandPacket, Cdc2ReplyPacket,
        ecmds::{CON_COMP_CTRL, USER_READ, CON_RADIO_CONFIGURE},
    },
};

pub type ConfigureRadioPacket = Cdc2CommandPacket<CON_CDC,CON_RADIO_CONFIGURE,ConfigureRadioPayload>;
pub type ConfigureRadioReplyPacket = Cdc2ReplyPacket<CON_CDC, CON_RADIO_CONFIGURE, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConfigureRadioPayload {
    //7 is the only value anything of importance uses.
    //(the radio doesn't even _seem to care_ if you request otherwise)
    pub con_types: u8,
    pub chan_type: u8,
    pub chan_num: u8,
    pub remote_ssn : u32,
    pub local_ssn :u32,
}

impl Encode for ConfigureRadioPayload {
    fn size(&self) -> usize {
        3+4+4
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.con_types;
        data[1] = self.chan_type;
        data[2] = self.chan_num;
        self.remote_ssn.encode(&mut data[3..]);
        self.local_ssn.encode(&mut data[7..]);
    }
}

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
    fn size(&self) -> usize {
        2 + self.write.as_ref().map(|write| write.size()).unwrap_or(0)
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.channel;

        if let Some(write) = &self.write {
            data[1] = write.size() as u8;
            write.encode(&mut data[2..]);
        } else {
            data[1] = 0;
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserDataReplyPayload {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Bytes read from stdout.
    pub data: Option<String>,
}
impl Decode for UserDataReplyPayload {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let channel = u8::decode(data)?;
        let data_len = data.len();

        let read = if data_len > 0 {
            Some({
                let mut utf8 = vec![];

                for _ in 0..data_len {
                    let byte = u8::decode(data)?;

                    if byte == 0 {
                        break;
                    }

                    utf8.push(byte);
                }

                core::str::from_utf8(&utf8)
                    .map_err(|e| DecodeError::new::<Self>(e.into()))?
                    .to_string()
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
#[repr(u8)]
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
    fn size(&self) -> usize {
        5
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.match_mode as u8;
        self.match_time.encode(&mut data[1..]);
    }
}
