//! VEXos system packets.

use core::u8;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    Decode, DecodeError, DecodeWithLength, Encode, FixedString, Version,
    cdc::cmds::USER_CDC,
    cdc2::{
        Cdc2CommandPacket, Cdc2ReplyPacket,
        ecmds::{
            DEV_STATUS, FDT_STATUS, LOG_READ, LOG_STATUS, RADIO_STATUS, SYS_C_INFO_14,
            SYS_C_INFO_58, SYS_DASH_SEL, SYS_DASH_TOUCH, SYS_FLAGS, SYS_KV_LOAD, SYS_KV_SAVE,
            SYS_SCREEN_CAP, SYS_STATUS, SYS_USER_PROG,
        },
    },
    decode::DecodeErrorKind,
};

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
            } => None,
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
            Err(e) if e.kind() == DecodeErrorKind::UnexpectedEnd => None,
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
            Err(e) if e.kind() == DecodeErrorKind::UnexpectedEnd => None,
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
        self.key.to_string().encode(data);
        self.value.to_string().encode(&mut data[self.key.size()..]);
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

pub type CatalogSlot1To4InfoPacket = Cdc2CommandPacket<USER_CDC, SYS_C_INFO_14, ()>;
pub type CatalogSlot1To4InfoReplyPacket =
    Cdc2CommandPacket<USER_CDC, SYS_C_INFO_14, SlotInfoPayload>;
pub type CatalogSlot5To8InfoPacket = Cdc2CommandPacket<USER_CDC, SYS_C_INFO_58, ()>;
pub type CatalogSlot5To8InfoReplyPacket =
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum DashScreen {
    /// Home screen
    Home = 0,

    /// Devices -> Battery
    Battery = 1,

    /// Devices -> Motor (program running)
    // Motor = 2,

    /// Unused Test Device
    Led = 3,

    /// Program -> Match
    Match = 4,

    /// Program -> Timed Run
    TimedRun = 5,

    /// Program -> Wiring
    Wiring = 6,

    /// Devices -> Partner
    // Controller2 = 7,

    /// Devices -> Radio
    Radio = 8,

    /// Devices -> Controller 1
    Controller1 = 9,

    /// Devices -> Brain
    Brain = 10,

    /// Devices -> Camera
    // Camera = 11,

    /// Devices -> Three-wire Ports
    // ThreeWire = 12,

    /// Program -> Run
    Running = 13,

    /// Program -> Controls
    ControlsA = 14,

    /// Default Drive Program
    Drive = 15,

    /// Devices Menu
    Devices = 16,

    /// Home -> User Folder
    UserFolder = 17,

    /// Home -> VEX Folder
    VexFolder = 18,

    /// Home -> Settings
    Settings = 19,

    /// Development Menu (be careful!!!)
    Config = 20,

    /// Settings -> Language (also shown on first boot)
    Language = 21,

    /// Drive -> Reverse
    MotorReverse = 22,

    /// Confirmation Screen (used to confirm a bunch of different settings changes)
    Confirm = 23,

    /// Program Menu
    UserProgram = 24,

    /// Shutdown Screen
    Off = 25,

    /// User controls for Controller 2 (Partner)
    Controller2Mapping = 26,

    /// Development Menu (be careful!!!)
    Config2 = 27,

    /// Error/Alert Screens
    Alert = 28,

    /// User controls for Controller 2 (Master)
    Controller1Mapping = 29,

    /// Drive -> Controls
    ControlsB = 30,

    /// Drive -> Controls
    ControlsC = 31,

    /// Drive -> Controls
    ControlsD = 32,

    /// (UNUSED) Multiplayer Match Screen
    Multiplayer = 33,

    /// Devices -> Brain -> Event Log
    EventLog = 34,

    /// Devices -> Motor (no program running)
    Motor2 = 35,

    // Test = 36,
    /// Program -> Wiring
    UserWiring = 40,

    /// Clawbot Program Run Screen
    Clawbot = 41,

    /// Settings -> Regulatory
    About = 42,

    /// Settings -> Language -> (more)
    Language2 = 43,

    /// Devices -> Camera -> Change Color
    Colors = 45,

    /// Devices -> Vision -> Select Signature
    SelectSignature = 46,

    /// (unknown)
    LogInfo = 47,
    // Abs = 48,
    // Imu = 49,
    // Color = 50,
    // Magnet = 51,
    // Distance = 52,
    // DistanceDev = 53,
    // Gps = 54,
    // AiCamera = 55,
    // LightTower = 56,
    // Arm = 57,
    // AiVision = 58,
    // Pneumatic = 59,
}

pub type DashTouchPacket = Cdc2CommandPacket<USER_CDC, SYS_DASH_TOUCH, DashTouchPayload>;
pub type DashTouchReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_DASH_TOUCH, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DashTouchPayload {
    pub x: u16,
    pub y: u16,
    /// 1 for pressing, 0 for released
    pub pressing: u16,
}
impl Encode for DashTouchPayload {
    fn size(&self) -> usize {
        6
    }

    fn encode(&self, data: &mut [u8]) {
        self.x.encode(data);
        self.y.encode(&mut data[2..]);
        self.pressing.encode(&mut data[4..]);
    }
}

pub type DashSelectPacket = Cdc2CommandPacket<USER_CDC, SYS_DASH_SEL, DashSelectPayload>;
pub type DashSelectReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_DASH_SEL, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DashSelectPayload {
    pub screen: DashScreen,

    /// This serves as a generic argument to the dash
    /// screen to select its "variant". It's named this
    /// because it's used to select a specific port number
    /// on a device screen.
    pub port: u8,
}
impl Encode for DashSelectPayload {
    fn size(&self) -> usize {
        2
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.screen as _;
        data[1] = self.port;
    }
}

pub type ScreenCapturePacket = Cdc2CommandPacket<USER_CDC, SYS_SCREEN_CAP, ScreenCapturePayload>;
pub type ScreenCaptureReplyPacket = Cdc2ReplyPacket<USER_CDC, SYS_SCREEN_CAP, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ScreenCapturePayload {
    /// Optionally, a specific LogiCVC layer to capture.
    pub layer: Option<u8>,
}
impl Encode for ScreenCapturePayload {
    fn size(&self) -> usize {
        if self.layer.is_some() { 1 } else { 0 }
    }

    fn encode(&self, data: &mut [u8]) {
        if let Some(layer) = self.layer {
            data[0] = layer;
        }
    }
}

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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
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
                return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                    name: "DeviceType",
                    value,
                    expected: &[
                        0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 26, 27, 28,
                        29, 30, 64, 70, 71, 128, 129, 255,
                    ],
                }));
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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let port = u8::decode(data)?;
        let device_type = DeviceType::decode(data)?;
        let status = u8::decode(data)?;
        let beta_version = u8::decode(data)?;
        let version = u16::decode(data)?;
        let boot_version = u16::decode(data)?;

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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let count = u8::decode(data)?;
        let devices = Vec::decode_with_len(data, count as _)?;
        Ok(Self { count, devices })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FdtStatus {
    pub count: u8,
    pub files: Vec<Fdt>,
}
impl Decode for FdtStatus {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let count = u8::decode(data)?;
        let entries = Vec::decode_with_len(data, count as _)?;

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
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let index = u8::decode(data)?;
        let fdt_type = u8::decode(data)?;
        let status = u8::decode(data)?;
        let beta_version = u8::decode(data)?;
        let version = u16::decode(data)?;
        let boot_version = u16::decode(data)?;

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
#[repr(u8)]
pub enum ConnectionDeviceType {
    NoConnection = 0,
    V5ControllerBluetooth = 2,
    V5ControllerVEXNet = 4,
    //speculated
    EXPControllerBluetooth = 6,
    //On a brain, a computer. On a controller, a brain.
    Host = 7,
}
impl Decode for ConnectionDeviceType {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let value = u8::decode(data)?;
        Ok(match value {
            0 => ConnectionDeviceType::NoConnection,
            2 => ConnectionDeviceType::V5ControllerBluetooth,
            4 => ConnectionDeviceType::V5ControllerVEXNet,
            6 => ConnectionDeviceType::EXPControllerBluetooth,
            7 => ConnectionDeviceType::Host,
            _ => {
                return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                    name: "DeviceType",
                    value,
                    expected: &[
                        0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 26, 27, 28,
                        29, 30, 64, 70, 71, 128, 129, 255,
                    ],
                }));
            }
        })
    }
}


#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RadioStatus {
    /// 0 = No controller, 4 = Controller connected (UNCONFIRMED)
    pub device: ConnectionDeviceType,
    /// From 0 to 100
    pub quality: u16,
    /// Probably RSSI (UNCONFIRMED)
    pub strength: i16,
    /// Vexnet3: 5 = download, 9 = reconnecting, anything lower than 53 is pit, else comp (there are a bunch)
    /// Bluetooth: MTU (typically around 240-250)
    pub channel: u8,
    /// Vexnet3: TDMA frame timeslot.
    /// Bluetooth: INT
    pub timeslot: u8,
}
impl Decode for RadioStatus {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let device = ConnectionDeviceType::decode(data)?;
        let quality = u16::decode(data)?;
        let strength = i16::decode(data)?;
        let channel = u8::decode(data)?;
        let timeslot = u8::decode(data)?;

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
