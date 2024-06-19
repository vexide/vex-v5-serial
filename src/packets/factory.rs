//! Factory Control

use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

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

pub struct FactoryStatus {
    pub status: u8,
    pub percent: u8,
}

pub type GetFdtStatusPacket = Cdc2CommandPacket<0x56, 0x23, ()>;
pub type GetFdtStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x23, FdtStatus>;

pub type GetFactoryStatusPacket = Cdc2CommandPacket<0x56, 0xF1, ()>;
pub type GetFactoryStatusReplyPacket = Cdc2ReplyPacket<0x56, 0xF1, FactoryStatus>;

pub type FactoryEnablePacket = Cdc2CommandPacket<0x56, 0xFF, FactoryEnablePayload>;
pub type FactoryEnableReplyPacket = Cdc2CommandPacket<0x56, 0xFF, ()>;

pub struct FactoryEnablePayload(pub [u8; 4]);

impl FactoryEnablePayload {
    pub const FACTORY_ENABLE_BYTES: [u8; 4] = [77, 76, 75, 74];

    pub const fn new() -> Self {
        Self(Self::FACTORY_ENABLE_BYTES)
    }
}