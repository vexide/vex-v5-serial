use std::time::Duration;

use vex_v5_serial::{
    Connection,
    protocol::{
        FixedString,
        cdc2::system::{KeyValueLoadPacket, KeyValueSavePacket},
    },
    serial::{self, SerialError},
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    let old_teamnumber = connection
        .handshake(
            KeyValueLoadPacket {
                key: FixedString::new("teamnumber").unwrap(),
            },
            Duration::from_millis(100),
            2,
        )
        .await??
        .value;

    // Set the team number on the brain
    connection
        .handshake(
            KeyValueSavePacket {
                key: FixedString::new("teamnumber")?,
                value: FixedString::new("vexide")?,
            },
            Duration::from_millis(100),
            2,
        )
        .await??;

    let new_teamnumber = connection
        .handshake(
            KeyValueLoadPacket {
                key: FixedString::new("teamnumber").unwrap(),
            },
            Duration::from_millis(100),
            2,
        )
        .await??
        .value;

    println!("{} -> {}", old_teamnumber.as_str(), new_teamnumber.as_str());

    Ok(())
}
