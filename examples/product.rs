use std::time::Duration;

use vexv5_serial::packets::system::{GetSystemVersionPacket, GetSystemVersionReplyPacket};

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
    let vex_ports = vexv5_serial::connection::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    device
        .send_packet(GetSystemVersionPacket::new(()))
        .await
        .unwrap();
    let response = device
        .recieve_packet::<GetSystemVersionReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap();

    println!("{:?}", response.payload.product_type);
}
