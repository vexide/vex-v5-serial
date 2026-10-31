use std::time::Duration;

use log::info;
use macro_rules_attribute::apply;
use vex_serial::{
    AsyncConnection,
    protocol::cdc2::system::DeviceStatusPacket,
    serial::{self, SerialError},
};

#[apply(smol_macros::main!)]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect_async()?;

    let status = connection
        .handshake(DeviceStatusPacket {}, Duration::from_millis(500), 10)
        .await??;

    for device in status.devices {
        info!("{:?} on port: {}", device.device_type, device.port);
    }

    Ok(())
}
