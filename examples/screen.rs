use std::time::Duration;

use vex_v5_serial::{
    Connection,
    protocol::cdc2::system::{
        DashScreen, DashSelectPacket, DashSelectPayload, DashSelectReplyPacket,
    },
    serial::{self, SerialError},
};

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
        .handshake::<DashSelectReplyPacket>(
            Duration::from_millis(500),
            10,
            DashSelectPacket::new(DashSelectPayload {
                screen: DashScreen::Settings,
                port: 0,
            }),
        )
        .await?
        .payload?;

    Ok(())
}
