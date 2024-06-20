use super::{cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket}, Decode};

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
impl Decode for DeviceStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, super::DecodeError>
       {
        let mut data = data.into_iter();
        let port = u8::decode(&mut data)?;
        let device_type = u8::decode(&mut data)?;
        let status = u8::decode(&mut data)?;
        let beta_version = u8::decode(&mut data)?;
        let version = u16::decode(&mut data)?;
        let boot_version = u16::decode(&mut data)?;
        Ok(Self { port, device_type, status, beta_version, version, boot_version })
    }
}

pub type GetDeviceStatusPacket = Cdc2CommandPacket<0x56, 0x21, ()>;
pub type GetDeviceStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x21, GetDeviceStatusReplyPayload>;

pub struct GetDeviceStatusReplyPayload {
    /// Number of elements in the following array.
    pub count: u8,
    pub devices: Vec<DeviceStatus>,
}
impl Decode for GetDeviceStatusReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, super::DecodeError> {
        let mut data = data.into_iter();
        let count = u8::decode(&mut data)?;
        let devices = Vec::decode(&mut data)?;
        Ok(Self { count, devices })
    }
}