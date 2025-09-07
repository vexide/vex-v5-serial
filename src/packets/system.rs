use super::{
    cdc::{
        cmds::{QUERY_1, SYSTEM_VERSION, USER_CDC},
        CdcCommandPacket, CdcReplyPacket,
    },
    cdc2::{
        ecmds::{
            FILE_USER_STAT, LOG_READ, LOG_STATUS, SYS_C_INFO_14, SYS_C_INFO_58, SYS_FLAGS,
            SYS_KV_LOAD, SYS_KV_SAVE, SYS_STATUS, SYS_USER_PROG,
        },
        Cdc2CommandPacket, Cdc2ReplyPacket,
    },
    file::FileVendor,
};
use crate::{
    decode::{Decode, DecodeError, SizedDecode},
    encode::{Encode, EncodeError},
    string::FixedString,
    version::Version,
};
use bitflags::bitflags;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u16)]
pub enum ProductType {
    Brain = 0x10,
    Controller = 0x11,
}
impl Decode for ProductType {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let _unknown = u8::decode(&mut data)?;
        let val = u8::decode(data)?;
        match val {
            0x10 => Ok(Self::Brain),
            0x11 => Ok(Self::Controller),
            v => Err(DecodeError::UnexpectedValue {
                value: v,
                expected: &[0x10, 0x11],
            }),
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

pub struct SystemFlags {
    /// Bit mask.
    /// From left to right:
    /// no.1 to no.8 bit - Page index
    /// no.12 bit = Radio Data mode on
    /// no.14 bit = Brain button double clicked
    /// no.15 bit = Battery is charging
    /// no.17 bit = Brain button clicked
    /// no.18 bit = Is VexNet mode
    /// no.19 bit = Has partner controller
    /// no.22 bit = Radio connected
    /// no.23 bit = Radio available
    /// no.24 bit = Controller tethered
    /// no.30 bit = Page changed
    /// no.32 bit = Device added/removed
    /// (RESEARCH NEEDED)
    pub flags: u32,

    /// Battery percent = First four bits * 8
    /// Controller battery percent = Last four bits * 8
    pub byte_1: u8,

    /// Radio quality = First four bits * 8
    /// Partner controller battery percent = Last four bits * 8
    pub byte_2: u8,

    /// The current program slot number, 0 means not in a program.
    /// 129 = ClawBot program
    /// 145 = Driver program
    pub current_program: u8,
}
impl Decode for SystemFlags {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let flags = u32::decode(&mut data)?;
        let byte_1 = u8::decode(&mut data)?;
        let byte_2 = u8::decode(&mut data)?;
        let current_program = u8::decode(&mut data)?;

        Ok(Self {
            flags,
            byte_1,
            byte_2,
            current_program,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemStatus {
    pub unknown: u8,
    pub system_version: Version,
    pub cpu0_version: Version,
    pub cpu1_version: Version,
    pub touch_version: Version,
    pub details: Option<SystemDetails>,
}
impl Decode for SystemStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let unknown = u8::decode(&mut data)?;
        let system_version = Version::decode(&mut data)?;
        let cpu0_version = Version::decode(&mut data)?;
        let cpu1_version = Version::decode(&mut data)?;

        // This version is little endian for some reason
        let touch_beta = u8::decode(&mut data)?;
        let touch_build = u8::decode(&mut data)?;
        let touch_minor = u8::decode(&mut data)?;
        let touch_major = u8::decode(&mut data)?;
        let touch_version = Version {
            major: touch_major,
            minor: touch_minor,
            build: touch_build,
            beta: touch_beta,
        };
        let details = Option::<SystemDetails>::decode(&mut data)?;

        Ok(Self {
            unknown,
            system_version,
            cpu0_version,
            cpu1_version,
            touch_version,
            details,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemDetails {
    pub unique_id: u32,

    /// (RESEARCH NEEDED)
    pub flags_1: u16,

    /// Bit mask.
    /// From left to right:
    /// no.1 bit = Is master controller charging
    /// no.2 bit = Is autonomous mode
    /// no.3 bit = Is disabled
    /// no.4 bit = Field controller connected
    /// (RESEARCH NEEDED)
    pub flags_2: u16,

    /// Bit mask.
    /// From left to right:
    /// no.1 to 4 bit = Language index, check out setting/language page
    /// no.6 bit = Is white theme
    /// no.8 bit = Is rotation normal
    /// no.14 bit = Ram boot loader active
    /// no.15 bit = Rom boot loader active
    /// no.16 bit = Is event brain/ Is field control signal from serial
    /// (RESEARCH NEEDED)
    pub flags_3: u16,
    pub unknown: u16,
    pub golden_version: Version,
    pub nxp_version: Option<Version>,
}
impl Decode for SystemDetails {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let unique_id = u32::decode(&mut data)?;
        let flags_1 = u16::decode(&mut data)?;
        let flags_2 = u16::decode(&mut data)?;
        let flags_3 = u16::decode(&mut data)?;
        let unknown = u16::decode(&mut data)?;
        let golden_version = Version::decode(&mut data)?;
        let nxp_version = Option::<Version>::decode(&mut data)?;

        Ok(Self {
            unique_id,
            flags_1,
            flags_2,
            flags_3,
            unknown,
            golden_version,
            nxp_version,
        })
    }
}

pub type SystemFlagsPacket = Cdc2CommandPacket<USER_CDC, SYS_FLAGS, ()>;
pub type SystemFlagsReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_FLAGS, SystemFlags>;

pub type SystemStatusPacket = Cdc2CommandPacket<USER_CDC, SYS_STATUS, ()>;
pub type SystemStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_STATUS, SystemStatus>;

pub type SystemVersionPacket = CdcCommandPacket<SYSTEM_VERSION, ()>;
pub type SystemVersionReplyPacket = CdcReplyPacket<SYSTEM_VERSION, SystemVersionReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemVersionReplyPayload {
    pub version: Version,
    pub product_type: ProductType,
    pub flags: ProductFlags,
}
impl Decode for SystemVersionReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let version = Version::decode(&mut data)?;
        let product_type = ProductType::decode(&mut data)?;
        let flags = ProductFlags::from_bits_truncate(u8::decode(&mut data)?);

        Ok(Self {
            version,
            product_type,
            flags,
        })
    }
}

pub type Query1Packet = CdcCommandPacket<QUERY_1, ()>;
pub type Query1ReplyPacket = CdcReplyPacket<QUERY_1, Query1ReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Query1ReplyPayload {
    pub unknown_1: [u8; 4],
    /// bytes 0-3 unknown
    pub joystick_flag_1: u8,
    pub joystick_flag_2: u8,
    /// Theorized to be version related, unsure.
    pub brain_flag_1: u8,
    pub brain_flag_2: u8,
    pub unknown_2: [u8; 2], // bytes 8 and 9 unknown
    pub bootload_flag_1: u8,
    pub bootload_flag_2: u8,
}

impl Decode for Query1ReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let unknown_1 = <[u8; 4]>::decode(&mut data)?;
        let joystick_flag_1 = u8::decode(&mut data)?;
        let joystick_flag_2 = u8::decode(&mut data)?;
        let brain_flag_1 = u8::decode(&mut data)?;
        let brain_flag_2 = u8::decode(&mut data)?;
        let unknown_2 = <[u8; 2]>::decode(&mut data)?;
        let bootload_flag_1 = u8::decode(&mut data)?;
        let bootload_flag_2 = u8::decode(&mut data)?;

        Ok(Self {
            unknown_1,
            joystick_flag_1,
            joystick_flag_2,
            brain_flag_1,
            brain_flag_2,
            unknown_2,
            bootload_flag_1,
            bootload_flag_2,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LogEntry {
    /// (RESEARCH NEEDED)
    pub code: u8,

    /// The subtype under the description (RESEARCH NEEDED)
    pub log_type: u8,

    /// The type of the log message (RESEARCH NEEDED)
    pub description: u8,

    /// (RESEARCH NEEDED)
    pub spare: u8,

    /// How long (in milliseconds) after the brain powered on
    pub time: u32,
}
impl Decode for LogEntry {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let code = u8::decode(&mut data)?;
        let log_type = u8::decode(&mut data)?;
        let description = u8::decode(&mut data)?;
        let spare = u8::decode(&mut data)?;
        let time = u32::decode(&mut data)?;
        Ok(Self {
            code,
            log_type,
            description,
            spare,
            time,
        })
    }
}

pub type LogCountPacket = Cdc2CommandPacket<USER_CDC, LOG_STATUS, ()>;
pub type LogCountReplyPacket = Cdc2ReplyPacket<USER_CDC, LOG_STATUS, LogCountReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LogCountReplyPayload {
    pub unknown: u8,
    pub count: u32,
}
impl Decode for LogCountReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let unknown = u8::decode(&mut data)?;
        let count = u32::decode(&mut data)?;
        Ok(Self { unknown, count })
    }
}

/// For example: If the brain has 26 logs, from A to Z. With offset 5 and count 5, it returns [V, W, X, Y, Z]. With offset 10 and count 5, it returns [Q, R, S, T, U].
pub type LogReadPacket = Cdc2CommandPacket<USER_CDC, LOG_READ, LogReadPayload>;
pub type LogReadReplyPacket = Cdc2ReplyPacket<USER_CDC, LOG_READ, LogReadReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LogReadPayload {
    pub offset: u32,
    pub count: u32,
}
impl Encode for LogReadPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.offset.to_le_bytes());
        encoded.extend(self.count.to_le_bytes());
        Ok(encoded)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogReadReplyPayload {
    /// Size of each log item in bytes.
    pub log_size: u8,
    /// The offset number used in this packet.
    pub offset: u32,
    /// Number of elements in the following array.
    pub count: u16,
    pub entries: Vec<LogEntry>,
}
impl Decode for LogReadReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut data = data.into_iter();

        let log_size = u8::decode(&mut data)?;
        let offset = u32::decode(&mut data)?;
        let count = u16::decode(&mut data)?;
        let entries = Vec::sized_decode(&mut data, count)?;

        Ok(Self {
            log_size,
            offset,
            count,
            entries,
        })
    }
}

pub type KeyValueLoadPacket = Cdc2CommandPacket<USER_CDC, SYS_KV_LOAD, FixedString<31>>;
pub type KeyValueLoadReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_KV_LOAD, FixedString<255>>;

pub type KeyValueSavePacket = Cdc2CommandPacket<USER_CDC, SYS_KV_SAVE, KeyValueSavePayload>;
pub type KeyValueSaveReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_KV_SAVE, ()>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyValueSavePayload {
    pub key: FixedString<31>,
    pub value: FixedString<255>,
}
impl Encode for KeyValueSavePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();

        encoded.extend(self.key.as_ref().to_string().encode()?);
        encoded.extend(self.value.as_ref().to_string().encode()?);

        Ok(encoded)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Slot {
    /// The number in the file icon: 'USER???x.bmp'.
    pub icon_number: u16,
    pub name_length: u8,
    pub name: String,
}
impl Decode for Slot {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let icon_number = u16::decode(&mut data)?;
        let name_length = u8::decode(&mut data)?;
        let name = String::sized_decode(&mut data, (name_length - 1) as _)?;

        Ok(Self {
            icon_number,
            name_length,
            name,
        })
    }
}

pub type ProgramStatusPacket = Cdc2CommandPacket<USER_CDC, FILE_USER_STAT, ProgramStatusPayload>;
pub type ProgramStatusReplyPacket =
    Cdc2ReplyPacket<USER_CDC, FILE_USER_STAT, ProgramStatusReplyPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProgramStatusPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    /// The bin file name.
    pub file_name: FixedString<23>,
}
impl Encode for ProgramStatusPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];

        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ProgramStatusReplyPayload {
    /// A zero-based slot number.
    pub slot: u8,

    /// A zero-based slot number, always same as Slot.
    pub requested_slot: u8,
}

pub type ProgramSlot1To4InfoPacket = Cdc2CommandPacket<USER_CDC, SYS_C_INFO_14, ()>;
pub type ProgramSlot1To4InfoReplyPacket =
    Cdc2CommandPacket<USER_CDC, SYS_C_INFO_14, SlotInfoPayload>;
pub type ProgramSlot5To8InfoPacket = Cdc2CommandPacket<USER_CDC, SYS_C_INFO_58, ()>;
pub type ProgramSlot5To8InfoReplyPacket =
    Cdc2CommandPacket<USER_CDC, SYS_C_INFO_58, SlotInfoPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SlotInfoPayload {
    /// Bit Mask.
    ///
    /// `flags & 2^(x - 1)` = Is slot x used
    pub flags: u8,

    /// Individual Slot Data
    pub slots: Vec<Slot>,
}
impl Decode for SlotInfoPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let flags = u8::decode(&mut data)?;
        let slots = Vec::sized_decode(&mut data, 4)?;

        Ok(Self { flags, slots })
    }
}

pub type UserProgramControlPacket = Cdc2CommandPacket<USER_CDC, SYS_USER_PROG, ()>;
pub type UserProgramControlReplyPacket = Cdc2CommandPacket<USER_CDC, SYS_USER_PROG, ()>;
