//! Controller packets.

use alloc::{
    string::{String, ToString},
    vec,
};

use crate::{
    Decode, DecodeError, Encode, FixedString,
    cdc::{CdcCommand, CdcReply, cmds},
    cdc2::{
        Cdc2Ack, Cdc2Command, Cdc2Reply, cdc2_command_size, decode_cdc2_reply, ecmds,
        frame_cdc2_command,
    },
};

// MARK: UserDataPacket

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserDataPacket {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Write (stdin) bytes.
    pub write: Option<FixedString<224>>,
}
impl Encode for UserDataPacket {
    fn size(&self) -> usize {
        cdc2_command_size(2 + self.write.as_ref().map(|write| write.size()).unwrap_or(0))
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.channel;

            if let Some(write) = &self.write {
                data[1] = write.size() as u8;
                write.encode(&mut data[2..]);
            } else {
                data[1] = 0;
            }
        })
    }
}

impl CdcCommand for UserDataPacket {
    const CMD: u8 = cmds::USER_CDC;
    type Reply = Result<UserDataReplyPacket, Cdc2Ack>;
}

impl Cdc2Command for UserDataPacket {
    const ECMD: u8 = ecmds::USER_READ;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UserDataReplyPacket {
    /// stdio channel is 1, other channels unknown.
    pub channel: u8,

    /// Bytes read from stdout.
    pub data: Option<String>,
}
impl Decode for UserDataReplyPacket {
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

impl Decode for Result<UserDataReplyPacket, Cdc2Ack> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        decode_cdc2_reply::<Self, UserDataReplyPacket>(data)
    }
}

impl CdcReply for Result<UserDataReplyPacket, Cdc2Ack> {
    const CMD: u8 = cmds::USER_CDC;
    type Command = UserDataPacket;
}

impl Cdc2Reply for Result<UserDataReplyPacket, Cdc2Ack> {
    const ECMD: u8 = ecmds::USER_READ;
}

// MARK: CompetitionControlPacket

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionControlPacket {
    pub mode: CompetitionMode,

    /// Time in seconds that should be displayed on the controller
    pub time: u32,
}

impl Encode for CompetitionControlPacket {
    fn size(&self) -> usize {
        cdc2_command_size(5)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.mode as u8;
            self.time.encode(&mut data[1..]);
        })
    }
}

impl CdcCommand for CompetitionControlPacket {
    type Reply = Result<CompetitionControlReplyPacket, Cdc2Ack>;
    const CMD: u8 = cmds::CON_CDC;
}
impl Cdc2Command for CompetitionControlPacket {
    const ECMD: u8 = ecmds::CON_COMP_CTRL;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionControlReplyPacket {}

impl Decode for CompetitionControlReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

impl Decode for Result<CompetitionControlReplyPacket, Cdc2Ack> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        decode_cdc2_reply::<Self, CompetitionControlReplyPacket>(data)
    }
}

impl CdcReply for Result<CompetitionControlReplyPacket, Cdc2Ack> {
    const CMD: u8 = cmds::CON_CDC;
    type Command = CompetitionControlPacket;
}
impl Cdc2Reply for Result<CompetitionControlReplyPacket, Cdc2Ack> {
    const ECMD: u8 = ecmds::CON_COMP_CTRL;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompetitionMode {
    Driver = 8,
    Auto = 10,
    Disabled = 11,
}
