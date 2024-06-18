use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};

#[repr(u8)]
pub enum DashScreen {
    Home = 0,
    Battery = 1,
    Led = 3,
    MatchConfig = 4,
    MatchConfigMore = 5,
    Wiring = 6,
    Radio = 8,
    Brain = 10,
    RunProgram = 13,
    DriveProgramControlLeftMapping = 14,
    DriveProgramMenu = 15,
    Devices = 16,
    UserProgramFolder = 17,
    VexProgramFolder = 18,
    Settings = 19,
    ScaryConfiguration = 20,
    Language = 21,
    DriveMotorConfig = 22,
    ProgramMenu = 24,
    Shutdown = 25,
    Controller2Mapping = 26,
    ScaryConfigurationMore = 27,
    ConfirmXX = 28,
    Controller1Mapping = 29,
    DriveProgramControlDualMapping = 30,
    DriveProgramControlSplitMapping = 31,
    DriveProgramControlRightMapping = 32,
    Match24Players = 33,
    EventLog = 34,
    UserProgramWiring = 40,
    ClawbotProgramMenu = 41,
    About = 42,
    LanguageMore = 43,
    ObjectColor = 45,
    SignatureId = 46,
    LogData = 47,
}

pub type SendDashTouchPacket = Cdc2CommandPacket<0x56, 0x2a, SendDashTouchPayload>;
pub type SendDashTouchReplyPacket = Cdc2ReplyPacket<0x56, 0x2a, ()>;

pub struct SendDashTouchPayload {
    pub x: u16,
    pub y: u16, 
    /// 1 for pressing, 0 for released
    pub pressing: u16,
}

pub type SelectDashPacket = Cdc2CommandPacket<0x56, 0x2b, SelectDashPayload>;
pub type SelectDashReplyPacket = Cdc2ReplyPacket<0x56, 0x2b, ()>;

pub struct SelectDashPayload {
    pub screen: DashScreen,
    /// (RESEARCH NEEDED)
    pub port: u8,
}