use std::time::Duration;

use vexv5_serial::{
    connection::{Connection, serial},
    packets::system::{GetSystemVersionPacket, GetSystemVersionReplyPacket},
};

#[tokio::main]
async fn main() {
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices().unwrap();

    // Open a connection to the device
    let mut connection = devices[0].open(Duration::from_secs(30)).unwrap();

    let response = connection
        .packet_handshake::<GetSystemVersionReplyPacket>(
            Duration::from_millis(700),
            5,
            GetSystemVersionPacket::new(()),
        )
        .await
        .unwrap();

    println!("{:?}", response.payload.product_type);
}
