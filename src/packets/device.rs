use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    array::Array,
    decode::{Decode, DecodeError},
};

// This is copied from vex-sdk
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum DeviceType {
    NoSensor = 0,
    Motor = 2,
    Led = 3,
    AbsEncoder = 4,
    CrMotor = 5,
    Imu = 6,
    DistanceSensor = 7,
    Radio = 8,
    TetheredController = 9,
    Brain = 10,
    VisionSensor = 11,
    AdiExpander = 12,
    Res1Sensor = 13,
    Battery = 14,
    Res3Sensor = 15,
    OpticalSensor = 16,
    Magnet = 17,
    GpsSensor = 20,
    AicameraSensor = 26,
    LightTower = 27,
    ArmDevice = 28,
    AiVisionSensor = 29,
    Pneumatic = 30,
    BumperSensor = 0x40,
    GyroSensor = 0x46,
    SonarSensor = 0x47,
    GenericSensor = 128,
    GenericSerial = 129,
    UndefinedSensor = 255,
}
impl Decode for DeviceType {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let value = u8::decode(data)?;
        Ok(match value {
            0 => DeviceType::NoSensor,
            2 => DeviceType::Motor,
            3 => DeviceType::Led,
            4 => DeviceType::AbsEncoder,
            5 => DeviceType::CrMotor,
            6 => DeviceType::Imu,
            7 => DeviceType::DistanceSensor,
            8 => DeviceType::Radio,
            9 => DeviceType::TetheredController,
            10 => DeviceType::Brain,
            11 => DeviceType::VisionSensor,
            12 => DeviceType::AdiExpander,
            13 => DeviceType::Res1Sensor,
            14 => DeviceType::Battery,
            15 => DeviceType::Res3Sensor,
            16 => DeviceType::OpticalSensor,
            17 => DeviceType::Magnet,
            20 => DeviceType::GpsSensor,
            26 => DeviceType::AicameraSensor,
            27 => DeviceType::LightTower,
            28 => DeviceType::ArmDevice,
            29 => DeviceType::AiVisionSensor,
            30 => DeviceType::Pneumatic,
            0x40 => DeviceType::BumperSensor,
            0x46 => DeviceType::GyroSensor,
            0x47 => DeviceType::SonarSensor,
            128 => DeviceType::GenericSensor,
            129 => DeviceType::GenericSerial,
            255 => DeviceType::UndefinedSensor,
            _ => {
                return Err(DecodeError::UnexpectedValue {
                    value,
                    expected: &[
                        0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 26, 27, 28,
                        29, 30, 0x40, 0x46, 0x47, 128, 129, 255,
                    ],
                })
            }
        })
    }
}

pub struct DeviceStatus {
    /// 1-indexed smart port number. Port 22 is the internal ADI expander and Port 23 is the battery.
    pub port: u8,

    /// Following V5_DeviceType
    pub device_type: DeviceType,

    /// 1 = smart port device, 0 = otherwise. (UNCONFIRMED)
    pub status: u8,
    pub beta_version: u8,
    pub version: u16,
    pub boot_version: u16,
}
impl Decode for DeviceStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let port = u8::decode(&mut data)?;
        let device_type = DeviceType::decode(&mut data)?;
        let status = u8::decode(&mut data)?;
        let beta_version = u8::decode(&mut data)?;
        let version = u16::decode(&mut data)?;
        let boot_version = u16::decode(&mut data)?;
        Ok(Self {
            port,
            device_type,
            status,
            beta_version,
            version,
            boot_version,
        })
    }
}

pub type GetDeviceStatusPacket = Cdc2CommandPacket<0x56, 0x21, ()>;
pub type GetDeviceStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x21, GetDeviceStatusReplyPayload>;

pub struct GetDeviceStatusReplyPayload {
    /// Number of elements in the following array.
    pub count: u8,
    pub devices: Array<DeviceStatus>,
}
impl Decode for GetDeviceStatusReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let count = u8::decode(&mut data)?;
        let devices = Array::decode_with_len(&mut data, count as _)?;
        Ok(Self { count, devices })
    }
}
