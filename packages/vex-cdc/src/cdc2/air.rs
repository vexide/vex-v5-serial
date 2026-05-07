//! Vex AIR Drone & Controller packets

use crate::FixedString;

#[repr(u8)]
pub enum AirCompetitionType {
    VRC = 0x0,
    AIR = 0x1,
}

pub struct SetEventInfoPacket {
    pub name: FixedString<20>,
    pub comp_type: AirCompetitionType,
    pub year: u8, //this is the last 2 digits of a year, y2k style
    pub event_id: u16, //unsure how many bits exactly right now.
}

//this is a bitflag type - TODO 
//also appears this may (?) be difference from the drone sequencing
//states used by the competition statemachine
pub enum AirDroneState {

}

pub struct SetMatchStatePacket {
    pub drone_state: AirDroneState,
    //skip 3 bytes
    pub timer: u32,
}