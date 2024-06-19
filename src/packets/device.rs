use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

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

pub type GetDeviceStatusPacket = Cdc2CommandPacket<0x56, 0x21, ()>;
pub type GetDeviceStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x21, GetDeviceStatusReplyPayload>;

pub struct GetDeviceStatusReplyPayload {
    /// Number of elements in the following array.
    pub count: u8,
    pub devices: Vec<DeviceStatus>,
}