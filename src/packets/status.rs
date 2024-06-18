use super::{cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket}, Version};

pub struct RadioStatus {
    /// 0 = No controller, 4 = Controller connected (UNCONFIRMED)
    pub device: u8,
    /// From 0 to 100
    pub quality: u16,
    /// Always negative
    pub strength: i16,
    pub channel: i8,
    /// Latency between controller and brain (UNCONFIRMED)
    pub timeslot: i8,
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

pub struct DeviceStatus {
    /// The value starts from 1. Port 22 is the internal ADI and Port 23 is the battery.
    pub port: u8,

    /// Following V5_DeviceType
    pub device_type: u8,
    
    /// 1 = smart port device, 0 = otherwise. (UNCONFIRMED)
    pub status: u8,
    pub beta_version: u8,
    pub version: u16,
    pub boot_version: u16,
}

pub struct SystemStatus {
    pub ignored: u8,
    pub system_version: Version,
    pub cpu0_version: Version,
    pub cpu1_version: Version,
    /// NOTE: Little endian
    pub touch_version: Version,
    pub details: Option<SystemDetails>,
}

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
    pub ignored: u16,
    pub golden_version: Version,
    pub nxp_version: Option<Version>,
}

pub struct FdtStatus {
    pub count: u8,
    pub entries: Vec<Fdt>,
}

pub struct Fdt {
    pub index: u8,
    pub fdt_type: u8,
    pub status: u8,
    pub beta_version: u8,
    pub version: u16,
    pub boot_version: u16,
}

pub type GetSystemFlagsPacket = Cdc2CommandPacket<0x56, 0x20, ()>;
pub type GetSystemFlagsReplyPacket = Cdc2ReplyPacket<0x56, 0x20, SystemFlags>;

pub type GetDeviceStatusPacket = Cdc2CommandPacket<0x56, 0x21, ()>;
pub type GetDeviceStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x21, GetDeviceStatusReplyPayload>;

pub struct GetDeviceStatusReplyPayload {
    /// Number of elements in the following array.
    pub count: u8,
    pub devices: Vec<DeviceStatus>,
}

pub type GetSystemStatusPacket = Cdc2CommandPacket<0x56, 0x22, ()>;
pub type GetSystemStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x22, SystemStatus>;

pub type GetFdtStatusPacket = Cdc2CommandPacket<0x56, 0x23, ()>;
pub type GetFdtStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x23, FdtStatus>;

pub type GetRadioStatusPacket = Cdc2CommandPacket<0x56, 0x26, ()>;
pub type GetRadioStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x26, RadioStatus>;