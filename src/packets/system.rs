use std::u8;

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
    decode::{Decode, DecodeError, DecodeWithLength},
    encode::Encode,
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let data = <[u8; 2]>::decode(data)?;

        match data[1] {
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let flags = u32::decode(data)?;
        let byte_1 = u8::decode(data)?;
        let byte_2 = u8::decode(data)?;
        let current_program = u8::decode(data)?;

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
    /// Always zero as of VEXos 1.1.5
    pub reserved: u8,
    /// returns None when connected via controller
    pub system_version: Option<Version>,
    pub cpu0_version: Version,
    pub cpu1_version: Version,
    pub touch_version: Version,
    pub details: Option<SystemDetails>,
}
impl Decode for SystemStatus {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let reserved = u8::decode(data)?;
        let system_version = match Version::decode(data)? {
            Version { 
                major: 0,
                minor: 0,
                build: 0,
                beta: 0,
            } => {
                None
            },
            version => Some(version),
        };

        let cpu0_version = Version::decode(data)?;
        let cpu1_version = Version::decode(data)?;

        // This version is little endian for some reason
        let touch_version = Version {
            beta: u8::decode(data)?,
            build: u8::decode(data)?,
            minor: u8::decode(data)?,
            major: u8::decode(data)?,
        };

        let details = match SystemDetails::decode(data) {
            Ok(details) => Some(details),
            Err(DecodeError::UnexpectedEnd) => None,
            Err(e) => return Err(e),
        };

        Ok(Self {
            reserved,
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
    /// Unique ID for the Brain.
    pub ssn: u32,
    pub boot_flags: u32,
    pub system_flags: u32,
    pub golden_version: Version,
    pub nxp_version: Option<Version>,
}
impl Decode for SystemDetails {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let ssn = u32::decode(data)?;
        let boot_flags = u32::decode(data)?;
        let system_flags = u32::decode(data)?;
        let golden_version = Version::decode(data)?;
        let nxp_version = match Version::decode(data) {
            Ok(version) => Some(version),
            Err(DecodeError::UnexpectedEnd) => None,
            Err(e) => return Err(e),
        };

        Ok(Self {
            ssn,
            boot_flags,
            system_flags,
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
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

pub type Query1Packet = CdcCommandPacket<QUERY_1, ()>;
pub type Query1ReplyPacket = CdcReplyPacket<QUERY_1, Query1ReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Query1ReplyPayload {
    pub version_1: u32,
    pub version_2: u32,

    /// 0xFF = QSPI, 0 = NOT sdcard, other = sdcard (returns devcfg.MULTIBOOT_ADDR)
    pub boot_source: u8,

    /// Number of times this packet has been replied to.
    pub count: u8,
}

impl Decode for Query1ReplyPayload {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let code = u8::decode(data)?;
        let log_type = u8::decode(data)?;
        let description = u8::decode(data)?;
        let spare = u8::decode(data)?;
        let time = u32::decode(data)?;

        Ok(Self {
            code,
            log_type,
            description,
            spare,
            time,
        })
    }
}

pub type LogStatusPacket = Cdc2CommandPacket<USER_CDC, LOG_STATUS, ()>;
pub type LogStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, LOG_STATUS, LogStatusReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LogStatusReplyPayload {
    /// Always zero as of VEXos 1.1.5
    pub reserved_1: u8,

    /// Total number of recorded event logs.
    pub count: u32,

    /// Always zero as of VEXos 1.1.5
    pub reserved_2: u32,

    /// Always zero as of VEXos 1.1.5
    pub reserved_3: u32,

    /// Always zero as of VEXos 1.1.5
    pub reserved_4: u32,
}
impl Decode for LogStatusReplyPayload {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let reserved = u8::decode(data)?;
        let count = u32::decode(data)?;
        let reserved_2 = u32::decode(data)?;
        let reserved_3 = u32::decode(data)?;
        let reserved_4 = u32::decode(data)?;

        Ok(Self {
            reserved_1: reserved,
            count,
            reserved_2,
            reserved_3,
            reserved_4,
        })
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
    fn size(&self) -> usize {
        8
    }

    fn encode(&self, data: &mut [u8]) {
        self.offset.encode(data);
        self.count.encode(&mut data[4..]);
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let log_size = u8::decode(data)?;
        let offset = u32::decode(data)?;
        let count = u16::decode(data)?;
        let entries = Vec::decode_with_len(data, count as _)?;

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
    fn size(&self) -> usize {
        self.key.size() + self.value.size()
    }

    fn encode(&self, data: &mut [u8]) {
        self.key.as_ref().to_string().encode(data);
        self.value
            .as_ref()
            .to_string()
            .encode(&mut data[self.key.size()..]);
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let icon_number = u16::decode(data)?;
        let name_length = u8::decode(data)?;
        let name = String::decode_with_len(data, (name_length - 1) as _)?;

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
    /// Unused as of VEXos 1.1.5
    pub reserved: u8,
    /// The bin file name.
    pub file_name: FixedString<23>,
}
impl Encode for ProgramStatusPayload {
    fn size(&self) -> usize {
        2 + self.file_name.size()
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.vendor as _;
        data[1] = self.reserved;

        self.file_name.encode(&mut data[2..]);
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
    pub slots: [Slot; 4],
}

impl Decode for SlotInfoPayload {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let flags = u8::decode(data)?;
        let slots = <[Slot; 4]>::decode(data)?;

        Ok(Self { flags, slots })
    }
}

pub type ProgramControlPacket = Cdc2CommandPacket<USER_CDC, SYS_USER_PROG, ()>;
pub type ProgramControlReplyPacket = Cdc2CommandPacket<USER_CDC, SYS_USER_PROG, ()>;
