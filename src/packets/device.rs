use super::{
    cdc::cmds::USER_CDC,
    cdc2::{
        ecmds::{DEV_STATUS, FDT_STATUS, RADIO_STATUS},
        Cdc2CommandPacket, Cdc2ReplyPacket,
    },
};

use crate::decode::{Decode, DecodeError, SizedDecode};

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
                        29, 30, 64, 70, 71, 128, 129, 255,
                    ],
                })
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

pub type DeviceStatusPacket = Cdc2CommandPacket<USER_CDC, DEV_STATUS, ()>;
pub type DeviceStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, DEV_STATUS, DeviceStatusReplyPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeviceStatusReplyPayload {
    /// Number of elements in the following array.
    pub count: u8,
    pub devices: Vec<DeviceStatus>,
}
impl Decode for DeviceStatusReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let count = u8::decode(&mut data)?;
        let devices = Vec::sized_decode(&mut data, count as _)?;
        Ok(Self { count, devices })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FdtStatus {
    pub count: u8,
    pub files: Vec<Fdt>,
}
impl Decode for FdtStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let count = u8::decode(&mut data)?;
        let entries = Vec::sized_decode(&mut data, count as _)?;
        Ok(Self {
            count,
            files: entries,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Fdt {
    pub index: u8,
    pub fdt_type: u8,
    pub status: u8,
    pub beta_version: u8,
    pub version: u16,
    pub boot_version: u16,
}
impl Decode for Fdt {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let index = u8::decode(&mut data)?;
        let fdt_type = u8::decode(&mut data)?;
        let status = u8::decode(&mut data)?;
        let beta_version = u8::decode(&mut data)?;
        let version = u16::decode(&mut data)?;
        let boot_version = u16::decode(&mut data)?;

        Ok(Self {
            index,
            fdt_type,
            status,
            beta_version,
            version,
            boot_version,
        })
    }
}

pub type FdtStatusPacket = Cdc2CommandPacket<USER_CDC, FDT_STATUS, ()>;
pub type FdtStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, FDT_STATUS, FdtStatus>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RadioStatus {
    /// 0 = No controller, 4 = Controller connected (UNCONFIRMED)
    pub device: u8,
    /// From 0 to 100
    pub quality: u16,
    /// Probably RSSI (UNCONFIRMED)
    pub strength: i16,
    /// 5 = download, 31 = pit, 245 = bluetooth
    pub channel: u8,
    /// Latency between controller and brain (UNCONFIRMED)
    pub timeslot: u8,
}
impl Decode for RadioStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();

        let device = u8::decode(&mut data)?;
        let quality = u16::decode(&mut data)?;
        let strength = i16::decode(&mut data)?;
        let channel = u8::decode(&mut data)?;
        let timeslot = u8::decode(&mut data)?;

        Ok(Self {
            device,
            quality,
            strength,
            channel,
            timeslot,
        })
    }
}

pub type RadioStatusPacket = Cdc2CommandPacket<USER_CDC, RADIO_STATUS, ()>;
pub type RadioStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, RADIO_STATUS, RadioStatus>;
