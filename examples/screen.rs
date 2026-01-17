use std::time::Duration;

use vex_v5_serial::{
    Connection,
    protocol::cdc2::system::{DashScreen, DashSelectPacket},
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
        .handshake(
            DashSelectPacket {
                screen: DashScreen::Settings,
                port: 0,
            },
            Duration::from_millis(500),
            10,
        )
        .await??;

    Ok(())
}
