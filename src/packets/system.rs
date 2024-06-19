use super::{
    cdc::{CdcCommandPacket, CdcReplyPacket},
    cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
    Version,
};
use bitflags::bitflags;

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

pub struct SystemStatus {
    pub unknown: u8,
    pub system_version: Version,
    pub cpu0_version: Version,
    pub cpu1_version: Version,
    /// NOTE: Encoded as little endian
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
    pub unknown: u16,
    pub golden_version: Version,
    pub nxp_version: Option<Version>,
}

pub type GetSystemFlagsPacket = Cdc2CommandPacket<0x56, 0x20, ()>;
pub type GetSystemFlagsReplyPacket = Cdc2ReplyPacket<0x56, 0x20, SystemFlags>;

pub type GetSystemStatusPacket = Cdc2CommandPacket<0x56, 0x22, ()>;
pub type GetSystemStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x22, SystemStatus>;

pub type GetSystemVersionPacket = CdcCommandPacket<0xA4, ()>;
pub type GetSystemVersionReplyPacket = CdcCommandPacket<0xA4, Version>;

#[repr(u8)]
pub enum ProductType {
    Brain = 0x10,
    Controller = 0x11,
}

bitflags! {
    pub struct ProductFlags: u8 {
        /// Bit 1 is set when the controller is connected over a cable to the V5 Brain
        const CONNECTED_CABLE = 1 << 0; // From testing, this appears to be how it works.

        /// Bit 2 is set when the controller is connected over VEXLink to the V5 Brain.
        const CONNECTED_WIRELESS = 1 << 1;
    }
}

pub struct GetSystemVersionReplyPayload {
    pub version: Version,
    pub product_type: ProductType,
    pub flags: ProductFlags,
}

pub type Query1Packet = CdcCommandPacket<0x21, ()>;
pub type Query1ReplyPacket = CdcReplyPacket<0x21, Query1ReplyPayload>;

pub struct Query1ReplyPayload {
    pub unknown_1: [u8; 4], /// bytes 0-3 unknown
    pub joystick_flag_1: u8,
    pub joystick_flag_2: u8,
    /// Theorized to be version related, unsure.
    pub brain_flag_1: u8,
    pub brain_flag_2: u8,
    pub unknown_2: [u8; 2], // bytes 8 and 9 unknown
    pub bootload_flag_1: u8,
    pub bootload_flag_2: u8,
}