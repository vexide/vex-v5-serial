use std::time::Duration;

use log::info;
use vex_v5_serial::{
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    packets::device::{DeviceStatusPacket, DeviceStatusReplyPacket},
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

    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    let status = connection
        .handshake::<DeviceStatusReplyPacket>(
            Duration::from_millis(500),
            10,
            DeviceStatusPacket::new(()),
        )
        .await?
        .payload?;

    for device in status.devices {
        info!("{:?} on port: {}", device.device_type, device.port);
    }

    Ok(())
}
