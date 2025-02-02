use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::encode::{Encode, EncodeError};

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

pub type SendDashTouchPacket = Cdc2CommandPacket<86, 42, SendDashTouchPayload>;
pub type SendDashTouchReplyPacket = Cdc2ReplyPacket<86, 42, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SendDashTouchPayload {
    pub x: u16,
    pub y: u16,
    /// 1 for pressing, 0 for released
    pub pressing: u16,
}
impl Encode for SendDashTouchPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        encoded.extend(self.x.to_le_bytes());
        encoded.extend(self.y.to_le_bytes());
        encoded.extend(self.pressing.to_le_bytes());
        Ok(encoded)
    }
}

pub type SelectDashPacket = Cdc2CommandPacket<86, 43, SelectDashPayload>;
pub type SelectDashReplyPacket = Cdc2ReplyPacket<86, 43, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SelectDashPayload {
    pub screen: DashScreen,
    /// (RESEARCH NEEDED)
    pub port: u8,
}
impl Encode for SelectDashPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.screen as u8, self.port])
    }
}
