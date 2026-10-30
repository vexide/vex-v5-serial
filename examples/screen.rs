use std::time::Duration;

use macro_rules_attribute::apply;
use vex_serial::{
    Connection,
    protocol::cdc2::system::DashSelectPacket,
    serial::{self, SerialError},
};

#[apply(smol_macros::main!)]
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
    let mut connection = devices[0].connect()?;

    connection
        .handshake(
            DashSelectPacket {
                screen: 10,
                port: 0,
            },
            Duration::from_millis(500),
            10,
        )??;

    Ok(())
}
