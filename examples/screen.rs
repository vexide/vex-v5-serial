use std::time::Duration;

use vex_v5_serial::{
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    encode::{Encode, EncodeError},
    packets::{
        cdc2::Cdc2CommandPacket, screen::DashSelectReplyPacket,
    },
};

pub type DashSelectPacket = Cdc2CommandPacket<0x56, 0x2B, DashSelectPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DashSelectPayload {
    screen: u8,
    port: u8,
}
impl Encode for DashSelectPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.screen, self.port])
    }
}

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    connection
        .packet_handshake::<DashSelectReplyPacket>(
            Duration::from_millis(500),
            10,
            DashSelectPacket::new(DashSelectPayload { screen: 85, port: 83 }),
        )
        .await?
        .try_into_inner()
        .unwrap();

    Ok(())
}
