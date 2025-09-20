use super::{
    cdc::cmds::USER_CDC,
    cdc2::{
        ecmds::{SYS_DASH_SEL, SYS_DASH_TOUCH, SYS_SCREEN_CAP},
        Cdc2CommandPacket, Cdc2ReplyPacket,
    },
};

use crate::encode::Encode;

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
        if self.layer.is_some() {
            1
        } else {
            0
        }
    }

    fn encode(&self, data: &mut [u8]) {
        if let Some(layer) = self.layer {
            data[0] = layer;
        }
    }
}
