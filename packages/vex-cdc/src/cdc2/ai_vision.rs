//! AI Vision

use std::io::Read;

use crate::{
    Decode, DecodeError, DecodeErrorKind, Encode, FixedString,
    cdc::cmds,
    cdc2::{cdc2_command_size, ecmds, frame_cdc2_command},
    cdc2_pair, decode,
};

// MARK: Status Packet

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
            y: u16::decode(data)?,
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
            class_name: {
                *data = &data[6..];
                FixedString::<16>::decode(data)?
            },
        })
    }
}

// MARK: Settings Packet

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
    StatusOverlay = 0x20, //there's only one byte of data, so probably another control bitflag
    Reset = 0x80,   //no other values need to be set to use Reset.
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
    pub status_ovl_flags: u8,
    //skip *many* bytes
    pub debug_print_colorcodes: bool, //at 0x3D. Does not print to CDC
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
                self.model_flags,
            ]
            .encode(&mut data[0x3..]);
            data[0x9] = self.status_ovl_flags;
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

// MARK: Model Information Packet

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
    pub model_version_str: FixedString<0x1f>,
}
impl Decode for AI2ModelInfoReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            load_status: u8::decode(data)?,
            model_ident: u32::decode(data)?,
            model_version: u32::decode(data)?,
            model_name: FixedString::<0x1f>::decode(data)?,
            model_version_str: FixedString::<0x1f>::decode(data)?,
        })
    }
}

// MARK: Clear Model
cdc2_pair!(
    AI2ClearModelPacket => AI2ClearModelReplyPacket,
    cmds::USER_CDC,
    ecmds::AI2CAM_CLEAR
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2ClearModelPacket {}

impl Encode for AI2ClearModelPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2ClearModelReplyPacket {}

impl Decode for AI2ClearModelReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: Get Objects
cdc2_pair!(
    AI2GetObjectsPacket => AI2GetObjectsReplyPacket,
    cmds::USER_CDC,
    ecmds::AI2CAM_OBJECTS,
);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2GetObjectsPacket {}

impl Encode for AI2GetObjectsPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AI2DetectionType {
    Unknown = 0,
    Color = 1,
    Code = 2,
    Object = 4,
    Tag = 8,
    NoDetection = 255,
}

impl Decode for AI2DetectionType {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(match (u8::decode(data)?) {
            1 => Self::Color,
            2 => Self::Code,
            4 => Self::Object,
            8 => Self::Tag,
            255 => Self::NoDetection,
            _ => Self::Unknown,
        })
    }
}

//unions are used here, so this reply packet structure doesn't really represent the byte layout
const AI2_OBJECT_SIZE: usize = 0xF;

/// a Color or Code detected object
/// Note that X/Y/W/H are 12 bit values though they are stored in a full 16 bit type

// 3 16-bit unsigned words -> 4 12-bit values.
fn decode_u12(words: [u16; 3]) -> [u16; 4] {
    [
        words[0] & 0x0FFF,
        ((words[0] >> 12) & 0x000F) | ((words[1] << 4) & 0x0FF0),
        ((words[1] >> 8) & 0x00FF) | (words[2] << 8 & 0x0F00),
        (words[2] >> 4) & 0x0FFF,
    ]
}
fn u12_to_i12(num: u16) -> i16 {
    if num & 0x800 == 1 {
        (num as i16) - 0x1000
    } else {
        num as i16
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AI2ColorCodeObject {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub angle: f32,
    pub id: u8,
}

impl Decode for AI2ColorCodeObject {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        //data is 12 bit, packed into 3 sets of u16s.
        let words = <[u16; 3]>::decode(data)?;
        let angle_raw = u16::decode(data)?;

        //X/Y/W/H
        let bitpacked_values = decode_u12(words);

        //clean up
        let remaining_bytes = AI2_OBJECT_SIZE - (2 + 8);
        *data = &data[remaining_bytes..];

        Ok(Self {
            x: bitpacked_values[0],
            y: bitpacked_values[1],
            w: bitpacked_values[2],
            h: bitpacked_values[3],
            angle: (angle_raw as f32) / 100.,
            id: 0, //placeholder
        })
    }
}

/// a Model detected object
/// Note that X/Y/W/H are 12 bit values though they are stored in a full 16 bit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2ModelObject {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub score: u16,
    pub id: u8,
}

impl Decode for AI2ModelObject {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
         //data is 12 bit, packed into 3 sets of u16s.
        let words = <[u16; 3]>::decode(data)?;
        let score = u16::decode(data)?;

        //X/Y/W/H
        let bitpacked_values = decode_u12(words);

        //clean up
        let remaining_bytes = AI2_OBJECT_SIZE - (2 + 8);
        *data = &data[remaining_bytes..];

        Ok(Self {
            x: bitpacked_values[0],
            y: bitpacked_values[1],
            w: bitpacked_values[2],
            h: bitpacked_values[3],
            score,
            id: 0, //placeholder
        })
    }
}

/// An April-Tag detected object
/// all values are in a signed 12 bit range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AI2TagObject {
    pub x0: i16,
    pub x1: i16,
    pub x2: i16,
    pub x3: i16,
    pub y0: i16,
    pub y1: i16,
    pub y2: i16,
    pub y3: i16,
    pub id: u8,
}

impl Decode for AI2TagObject {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let words_a = <[u16; 3]>::decode(data)?;
        let words_b = <[u16; 3]>::decode(data)?;
        //x0/y0/x1/y1
        let mut vals_a = decode_u12(words_a);
        //x2/y2/x3/y3
        let mut vals_b = decode_u12(words_b);

        let remaining_bytes = AI2_OBJECT_SIZE - (2 + 12);
        *data = &data[remaining_bytes..];

        Ok(Self {
            x0: u12_to_i12(vals_a[0]),
            y0: u12_to_i12(vals_a[1]),
            x1: u12_to_i12(vals_a[2]),
            y1: u12_to_i12(vals_a[3]),

            x2: u12_to_i12(vals_b[0]),
            y2: u12_to_i12(vals_b[1]),
            x3: u12_to_i12(vals_b[2]),
            y3: u12_to_i12(vals_b[3]),

            id: 0, //placeholder
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AI2GetObjectsReplyPacket {
    pub num_objects: u8,
    pub color_objs: Vec<AI2ColorCodeObject>,
    pub tag_objs: Vec<AI2TagObject>,
    pub model_objs: Vec<AI2ModelObject>,
}

impl Decode for AI2GetObjectsReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let obj_count = u8::decode(data)?;
        let mut colors: Vec<AI2ColorCodeObject> = vec![];
        let mut tags: Vec<AI2TagObject> = vec![];
        let mut models: Vec<AI2ModelObject> = vec![];

        for _ in 0..obj_count {
            let id = u8::decode(data)?;
            match (AI2DetectionType::decode(data)?) {
                AI2DetectionType::Code | AI2DetectionType::Color => {
                    let mut obj = AI2ColorCodeObject::decode(data)?;
                    obj.id = id;
                    colors.push(obj);
                }
                AI2DetectionType::Tag => {
                    let mut obj = AI2TagObject::decode(data)?;
                    obj.id = id;
                    tags.push(obj);
                }
                AI2DetectionType::Object => {
                    let mut obj = AI2ModelObject::decode(data)?;
                    obj.id = id;
                    models.push(obj);
                }
                invalid => {
                    return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                        name: "AI2DetectionType",
                        value: invalid as u8,
                        expected: &[1, 2, 4, 8],
                    }));
                }
            }
        }

        Ok(Self {
            num_objects: obj_count,
            color_objs: colors,
            tag_objs: tags,
            model_objs: models,
        })
    }
}
