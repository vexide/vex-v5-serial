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
    ecmds::AI2CAM_STATUS
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