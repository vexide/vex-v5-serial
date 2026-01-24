//! Simple CDC packets.

use crate::{
    decode::{Decode, DecodeError, DecodeErrorKind},
    encode::Encode,
    varint::VarU16,
    version::Version,
};

use alloc::vec::Vec;
use bitflags::bitflags;

/// CDC packet opcodes.
///
/// These are the byte values identifying the different CDC commands.
/// This module is non-exhaustive.
pub mod cmds {
    pub const ACK: u8 = 0x33;
    pub const QUERY_1: u8 = 0x21;
    pub const USER_CDC: u8 = 0x56;
    pub const CON_CDC: u8 = 0x58;
    pub const SYSTEM_VERSION: u8 = 0xA4;
    pub const EEPROM_ERASE: u8 = 0x31;
    pub const USER_ENTER: u8 = 0x60;
    pub const USER_CATALOG: u8 = 0x61;
    pub const FLASH_ERASE: u8 = 0x63;
    pub const FLASH_WRITE: u8 = 0x64;
    pub const FLASH_READ: u8 = 0x65;
    pub const USER_EXIT: u8 = 0x66;
    pub const USER_PLAY: u8 = 0x67;
    pub const USER_STOP: u8 = 0x68;
    pub const COMPONENT_GET: u8 = 0x69;
    pub const USER_SLOT_GET: u8 = 0x78;
    pub const USER_SLOT_SET: u8 = 0x79;
    pub const BRAIN_NAME_GET: u8 = 0x44;
}

/// Starting byte sequence for all device-bound CDC packets.
/// 
/// The fourth (0x47) byte may change depending on the intended device target, for example a
/// controller may internally use 0x4e.
pub const COMMAND_HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

/// Starting byte sequence used for all host-bound CDC packets.
pub const REPLY_HEADER: [u8; 2] = [0xAA, 0x55];

/// CDC (Simple) command packet.
///
/// A device-bound message containing a command identifier and an encoded payload.
/// Each packet begins with a 4-byte [`COMMAND_HEADER`], followed by the opcode,
/// and then an optional length-prefixed payload.
///
/// The payload type `P` must implement [`Encode`].
///
/// # Encoding
///
/// | Field     | Size   | Description |
/// |-----------|--------|-------------|
/// | `header`  | 4      | Must be [`COMMAND_HEADER`]. |
/// | `cmd`     | 1      | A [CDC command opcode](crate::cdc::cmds) indicating the type of packet. |
/// | `size`    | 1–2    | Size of `payload` encoded as a [`VarU16`]. |
/// | `payload` | n      | Encoded payload. |
pub trait CdcCommand: Encode {
    const CMD: u8;
    const HEADER: [u8; 4] = COMMAND_HEADER;

    type Reply: CdcReply;
}

/// CDC (Simple) reply packet.
///
/// A host-bound packet sent in response to a [`CdcCommandPacket`].  
/// Each reply consists of a 2-byte [`REPLY_HEADER`], the echoed command ID,
/// a variable-width length field, and the decoded payload.
///
/// The payload type `P` must implement [`Decode`].
///
/// # Encoding
///
/// | Field     | Size   | Description |
/// |-----------|--------|-------------|
/// | `header`  | 2      | Must be [`REPLY_HEADER`]. |
/// | `cmd`     | 1      | A [CDC command opcode](crate::cdc::cmds) indicating the type of command being replied to. |
/// | `size`    | 1–2    | Size of `payload` encoded as a [`VarU16`]. |
/// | `payload` | n      | Encoded payload. |
pub trait CdcReply: Decode {
    const CMD: u8;
    const HEADER: [u8; 2] = REPLY_HEADER;

    type Command: CdcCommand;
}

pub(crate) fn decode_cdc_reply_frame<R: CdcReply>(data: &mut &[u8]) -> Result<u16, DecodeError> {
    if <[u8; 2]>::decode(data)? != R::HEADER {
        return Err(DecodeError::new::<R>(DecodeErrorKind::InvalidHeader));
    }

    let cmd = u8::decode(data)?;
    if cmd != R::CMD {
        return Err(DecodeError::new::<R>(DecodeErrorKind::UnexpectedByte {
            name: "cmd",
            value: cmd,
            expected: &[R::CMD],
        }));
    }

    Ok(VarU16::decode(data)?.into_inner())
}

// MARK: Packets

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemVersionPacket {}

impl Encode for SystemVersionPacket {
    fn encode(&self, data: &mut [u8]) {
        Self::HEADER.encode(data);
        data[4] = Self::CMD;
    }

    fn size(&self) -> usize {
        5
    }
}

impl CdcCommand for SystemVersionPacket {
    const CMD: u8 = cmds::SYSTEM_VERSION;
    type Reply = SystemVersionReplyPacket;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemVersionReplyPacket {
    pub version: Version,
    pub product_type: ProductType,
    pub flags: ProductFlags,
}

impl Decode for SystemVersionReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        decode_cdc_reply_frame::<Self>(data)?;

        let version = Version::decode(data)?;
        let product_type = ProductType::decode(data)?;
        let flags = ProductFlags::from_bits_truncate(u8::decode(data)?);

        Ok(Self {
            version,
            product_type,
            flags,
        })
    }
}

impl CdcReply for SystemVersionReplyPacket {
    const CMD: u8 = cmds::SYSTEM_VERSION;
    type Command = SystemVersionPacket;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemAlivePacket {}

impl Encode for SystemAlivePacket {
    fn encode(&self, data: &mut [u8]) {
        Self::HEADER.encode(data);
        data[4] = Self::CMD;
    }

    fn size(&self) -> usize {
        5
    }
}

impl CdcCommand for SystemAlivePacket {
    const CMD: u8 = cmds::QUERY_1;
    type Reply = SystemAliveReplyPacket;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemAliveReplyPacket {
    pub version_1: u32,
    pub version_2: u32,

    /// 0xFF = QSPI, 0 = NOT sdcard, other = sdcard (returns devcfg.MULTIBOOT_ADDR)
    pub boot_source: u8,

    /// Number of times this packet has been replied to.
    pub count: u8,
}

impl Decode for SystemAliveReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        decode_cdc_reply_frame::<Self>(data)?;

        let version_1 = u32::decode(data)?;
        let version_2 = u32::decode(data)?;
        let boot_source = u8::decode(data)?;
        let count = u8::decode(data)?;

        Ok(Self {
            version_1,
            version_2,
            boot_source,
            count,
        })
    }
}

impl CdcReply for SystemAliveReplyPacket {
    const CMD: u8 = cmds::QUERY_1;
    type Command = SystemAlivePacket;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u16)]
pub enum ProductType {
    /// V5 Robot Brain (276-4810)
    V5Brain = 0x10,

    /// V5 Controller (276-4820)
    V5Controller = 0x11,

    /// Smart Field Controller (276-7577)
    V5EventBrain = 0x14,

    /// Unknown EXP Brain Variant
    ExpBrainVariant = 0x16,

    /// Unknown EXP Controller Variant
    ExpControllerVariant = 0x17,

    /// V5 GPS Sensor (276-7405)
    GpsSensor = 0x18,

    /// IQ Robot Brain (Generation 2) (228-6480)
    Iq2Brain = 0x20,

    /// IQ Controller (Generation 2) (228-6470)
    Iq2Controller = 0x21,

    /// EXP Robot Brain (280-7125)
    ExpBrain = 0x60,

    /// EXP Controller (280-7729)
    ExpController = 0x61,

    /// VEX AIM Coding Robot (249-8581)
    Aim = 0x70,

    /// AI Vision Sensor (276-8659, 228-9136)
    AiVision = 0x80,

    /// CTE Workcell Arm (234-8952)
    CteWorkcellArm = 0x90,

    /// VEX Air Controller
    AirController = 0xA1,
}

impl Decode for ProductType {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let data = <[u8; 2]>::decode(data)?;

        match data[1] {
            0x10 => Ok(Self::V5Brain),
            0x11 => Ok(Self::V5Controller),
            0x14 => Ok(Self::V5EventBrain),
            0x16 => Ok(Self::ExpBrainVariant),
            0x17 => Ok(Self::ExpControllerVariant),
            0x18 => Ok(Self::GpsSensor),
            0x20 => Ok(Self::Iq2Brain),
            0x21 => Ok(Self::Iq2Controller),
            0x60 => Ok(Self::ExpBrain),
            0x61 => Ok(Self::ExpController),
            0x70 => Ok(Self::Aim),
            0x80 => Ok(Self::AiVision),
            0x90 => Ok(Self::CteWorkcellArm),
            0xA1 => Ok(Self::AirController),
            v => Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                name: "ProductType",
                value: v,
                expected: &[
                    0x10, 0x11, 0x14, 0x16, 0x17, 0x18, 0x20, 0x21, 0x60, 0x61, 0x70, 0x80, 0x90, 0xA1,
                ],
            })),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct ProductFlags: u8 {
        /// Bit 1 is set when the controller is connected over a cable to the V5 Brain
        const CONNECTED_CABLE = 1 << 0; // From testing, this appears to be how it works.

        /// Bit 2 is set when the controller is connected over VEXLink to the V5 Brain.
        const CONNECTED_WIRELESS = 1 << 1;
    }
}
