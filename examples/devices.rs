use std::time::Duration;

use log::info;
use vexv5_serial::{
    connection::{serial, Connection},
    packets::device::{GetDeviceStatusPacket, GetDeviceStatusReplyPacket},
};

#[tokio::main]
async fn main() {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices().unwrap();

    // Open a connection to the device
    let mut connection = devices[0].open(Duration::from_secs(30)).unwrap();

    let devices = connection
        .packet_handshake::<GetDeviceStatusReplyPacket>(
            Duration::from_millis(500),
            10,
            GetDeviceStatusPacket::new(()),
        )
        .await
        .unwrap()
        .payload
        .try_into_inner()
        .unwrap();
    for device in devices.devices.into_inner() {
        info!("{:?} on port: {}", device.device_type, device.port);
    }
}
