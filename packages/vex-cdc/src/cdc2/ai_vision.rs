//! AI Vision

use crate::{
    Decode, DecodeError, DecodeErrorKind, Encode, FixedString,
    cdc::cmds,
    cdc2::{cdc2_command_size, ecmds, frame_cdc2_command},
    cdc2_pair,
};

cdc2_pair!(
    AI2StatusPacket => AI2StatusReplyPacket,
    cmds::USER_CDC,
    ecmds::AI2CAM_STATUS,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2Bound {
    pub x: u16,
    pub y: u16,
}

impl Decode for AI2Bound {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            x: u16::decode(data)?,
            y: u16::decode(data)?
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2StatusPacket {}

impl Encode for AI2StatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AI2StatusReplyPacket {
    pub msg_id: u8, //always 0x8F
    pub status: u8,
    pub temperature: u16, //convert to double and divide by 256.0 to get in C
    pub col_bounds: AI2Bound,
    pub tag_bounds: AI2Bound,
    pub obj_bounds: AI2Bound,
    pub mode: u8,
    /* Flags */
    pub enable_flg: u8,
    pub test_flg: u8,
    pub sensor_ctl: u8,
    pub model_ctl: u8,
    pub tags_ctl: u8,
    pub color_ctl: u8,
    //PAD: 0x1
    /* Object Counts */
    pub color_objs: u8,
    pub tag_objs: u8,
    pub model_objs: u8,
    //PAD: 0x1
    /* Performance Stats */
    pub color_fps: u8,
    pub tag_fps: u8,
    pub model_fps: u8,
    /* Other misc. data */
    pub color_match: u8,
    pub code_seq: u8,
    pub class_count: u8, //how many total model classes
    pub class_id: u8,    //for the string at the end
    pub tag_decimate: u8,
    /* AI Model Metadata */
    pub model_flags: u8, //unused
    pub model_id: u8,
    pub model_version: u8,
    //PAD: 6
    pub class_name: FixedString<16>,
    //smartport crc ommited
}

impl Decode for AI2StatusReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            msg_id: u8::decode(data)?,
            status: u8::decode(data)?,
            temperature: u16::decode(data)?,
            col_bounds: AI2Bound::decode(data)?,
            tag_bounds: AI2Bound::decode(data)?,
            obj_bounds: AI2Bound::decode(data)?,
            mode: u8::decode(data)?,
            enable_flg: u8::decode(data)?,
            test_flg: u8::decode(data)?,
            sensor_ctl: u8::decode(data)?,
            model_ctl: u8::decode(data)?,
            tags_ctl: u8::decode(data)?,
            color_ctl: u8::decode(data)?,
            //> there's a dummy byte in between here to avoid
            color_objs: { 
                *data = &data[1..];
                u8::decode(data)? 
            },
            tag_objs: u8::decode(data)?,
            model_objs: u8::decode(data)?,
            //> there's a dummy byte in between here to avoid
            color_fps: { 
                *data = &data[1..];
                u8::decode(data)? 
            },
            tag_fps: u8::decode(data)?,
            model_fps: u8::decode(data)?,
            //> there's a dummy byte in between here to avoid
            color_match: { 
                *data = &data[1..];
                u8::decode(data)? 
            },
            code_seq: u8::decode(data)?,
            class_count: u8::decode(data)?,
            class_id: u8::decode(data)?,
            tag_decimate: u8::decode(data)?,
            model_flags: u8::decode(data)?,
            model_id: u8::decode(data)?,
            model_version: u8::decode(data)?,
            //> there's 6 dummy bytes in between here to avoid
            class_name : {
                *data = &data[6..];
                FixedString::<16>::decode(data)?
            }
        })
    }
}

/* AI2CAM Settings */

cdc2_pair!(
    AI2SettingsPacket => AI2SettingsReplyPacket,
    cmds::USER_CDC,
    ecmds::AI2CAM_SETTINGS,
);

/// The AI2 Settings packet can potential set flags for multiple different
/// categories of field on the sensor. Setting the corresponding command flag
/// bit tells the sensor to read and use the corresponding values in the overall packet.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AI2SettingFlag {
    Mode = 0x1,
    Enable = 0x2,
    Test = 0x4,
    Sensor = 0x8,
    Model = 0x10,
    Unknown = 0x20, //there's only one byte of data, so probably another control bitflag
    Reset = 0x80, //no other values need to be set to use Reset. 
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2SettingsPacket {
    //pub msg_id: u8 //this can be set to any value; unused
    pub flags: AI2SettingFlag,
    //skip a byte
    pub enable_flags: u8,
    pub test_sigs: u8,
    pub sensor_sigs: u8,
    pub model_flags: u8,
    //skip 2 bytes
    pub unknown_flags: u8, //corresponds to AI2SettingFlag::Unknown 0x20
    //skip *many* bytes
    pub debug_print_colorcodes: bool //at 0x3D. Does not print to CDC
}

impl Encode for AI2SettingsPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0x40)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            data[1] = self.flags as u8;
            [
                self.enable_flags,
                self.test_sigs,
                self.sensor_sigs,
                self.model_flags
            ].encode(&mut data[0x3..]);
            data[0x9] = self.unknown_flags;
            data[0x3d] = self.debug_print_colorcodes as u8;
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2SettingsReplyPacket {}

impl Decode for AI2SettingsReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

/* AI2CAM Model Information */
cdc2_pair!(
    AI2ModelInfoPacket => AI2ModelInfoReplyPacket,
    cmds::USER_CDC,
    ecmds::AI2CAM_MODEL
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2ModelInfoPacket {}

impl Encode for AI2ModelInfoPacket {
    fn size(&self) -> usize {
        //technically this is the "extended" form of the packet (a variant exists that doesn't return the version string)
        //to recieve that version, simply send a zero-length payload.
        cdc2_command_size(1)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AI2ModelInfoReplyPacket {
    pub load_status: u8,
    pub model_ident: u32,
    pub model_version: u32,
    pub model_name: FixedString<0x1f>,
    pub model_version_str: FixedString<0x1f>
}
impl Decode for AI2ModelInfoReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            load_status: u8::decode(data)? ,
            model_ident: u32::decode(data)?,
            model_version: u32::decode(data)?,
            model_name: FixedString::<0x1f>::decode(data)?,
            model_version_str: FixedString::<0x1f>::decode(data)?,
        })
    }
}