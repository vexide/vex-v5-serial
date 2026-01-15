//! Controller packets.

use alloc::{
    string::{String, ToString},
    vec,
};

use crate::{
    Decode, DecodeError, Encode, FixedString,
    cdc::cmds,
    cdc2::{cdc2_command_size, ecmds, frame_cdc2_command},
    cdc2_pair,
};

// MARK: UserData

cdc2_pair!(
    UserDataPacket => UserDataReplyPacket,
    cmds::USER_CDC,
    ecmds::USER_READ
);

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

// MARK: CompetitionControl

cdc2_pair!(
    CompetitionControlPacket => CompetitionControlReplyPacket,
    cmds::CON_CDC,
    ecmds::CON_COMP_CTRL
);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitionControlReplyPacket {}

impl Decode for CompetitionControlReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompetitionMode {
    Driver = 8,
    Autonomous = 10,
    Disabled = 11,
}

// MARK: ConfigureRadio


cdc2_pair!(
    ConfigureRadioPacket => ConfigureRadioReplyPacket,
    cmds::CON_CDC,
    ecmds::CON_RADIO_CONFIGURE
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigureRadioPacket {
    //7 is the only value anything of importance uses.
    //(the radio doesn't even _seem to care_ if you request otherwise)
    pub con_types: u8,
    pub chan_type: u8,
    pub chan_num: u8,
    pub remote_ssn: u32,
    pub local_ssn: u32,
}

impl Encode for ConfigureRadioPacket {
    fn size(&self) -> usize {
        cdc2_command_size(3 + 4 + 4)
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.con_types;
        data[1] = self.chan_type;
        data[2] = self.chan_num;
        self.remote_ssn.encode(&mut data[3..]);
        self.local_ssn.encode(&mut data[7..]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigureRadioReplyPacket {}

impl Decode for ConfigureRadioReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}