//! Controller packets.

use alloc::{
    string::{String, ToString},
    vec,
};

use crate::{
    Decode, DecodeError, DecodeErrorKind, Encode, FixedString, cdc::cmds, cdc2::{cdc2_command_size, ecmds, frame_cdc2_command}, cdc2_pair
};

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

// MARK: SmartFieldData

cdc2_pair!(
    SmartFieldDataPacket => SmartFieldDataReplyPacket,
    cmds::CON_CDC,
    ecmds::CON_COMP_GET_SMARTFIELD
);

pub struct SmartFieldDataPacket {}

impl Encode for SmartFieldDataPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SmartFieldDataReplyPacket {
    pub rssi: u8,                         // 0x1
    pub controller_status: u16,           // 0x2-3
    pub radio_status: u16,                // 0x4-5
    pub field_status: u8,                 // 0x6
    pub timer: u8,                        // 0x7
    pub brain_battery: u8,                // 0x8
    pub primary_battery: u8,              // 0x9
    pub partner_battery: u8,              // 0xA
    pub brain_battery_capacity: u8,       // 0xB why is this separate from the cap?
    pub primary_buttons: u16,             // 0xC-D bitfield
    pub running_slot: u8,                 // 0xE dashboard mode is slot 0x11, drive is high up somewhere
    pub radio_type: ControllerRadioType,  // 0xF
    pub radio_channel: u8,                // 0x10
    pub radio_timeslot: u8,               // 0x11,
    pub team_number: FixedString<8>,      // 0x12+
    pub device_flags: u8,                 // 0x1c
    pub radio_quality: u8,                // 0x1d
                                          // smartport CRC omitted
}

impl Decode for SmartFieldDataReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            rssi: u8::decode(data)?,
            controller_status: u16::decode(data)?,
            radio_status: u16::decode(data)?,
            field_status: u8::decode(data)?,
            timer: u8::decode(data)?,
            brain_battery: u8::decode(data)?,
            primary_battery: u8::decode(data)?,
            partner_battery: u8::decode(data)?,
            brain_battery_capacity: u8::decode(data)?,
            primary_buttons: u16::decode(data)?,
            running_slot: u8::decode(data)?,
            radio_type: ControllerRadioType::decode(data)?,
            radio_channel: u8::decode(data)?,
            radio_timeslot: u8::decode(data)?,
            team_number: FixedString::<8>::decode(data)?,
            device_flags: data[2],
            radio_quality: data[3]
        })
    }
}

// MARK: ChangeRadioType

cdc2_pair!(
    ChangeRadioTypePacket => ChangeRadioTypeReplyPacket,
    cmds::CON_CDC,
    ecmds::CON_RADIO_CONTYPE
);

pub struct ChangeRadioTypePacket {
    pub radio_type: ControllerRadioType,
}

impl Encode for ChangeRadioTypePacket {
    fn size(&self) -> usize {
        cdc2_command_size(1)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[0] = self.radio_type as u8;
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeRadioTypeReplyPacket {}

impl Decode for ChangeRadioTypeReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ControllerRadioType {
    Vexnet3 = 0x1,
    Bluetooth = 0x2,
}
impl Decode for ControllerRadioType {
     fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let value = u8::decode(data)?;
        Ok(match value {
            1 => ControllerRadioType::Vexnet3,
            2 => ControllerRadioType::Bluetooth,
            _ => {
                return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                    name: "ControllerRadioType",
                    value,
                    expected: &[
                        1,2
                    ],
                }));
            }
        })
    }
}