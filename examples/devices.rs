use std::time::Duration;

use log::info;
use vexv5_serial::{
    connection::{serial, Connection, ConnectionError},
    packets::device::{GetDeviceStatusPacket, GetDeviceStatusReplyPacket},
};

#[tokio::main]
async fn main() -> Result<(), ConnectionError> {
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

    let status = connection
        .packet_handshake::<GetDeviceStatusReplyPacket>(
            Duration::from_millis(500),
            10,
            GetDeviceStatusPacket::new(()),
        )
        .await?
        .payload
        .try_into_inner()?;

    for device in status.devices.into_inner() {
        info!("{:?} on port: {}", device.device_type, device.port);
    }

    Ok(())
}
