use std::time::Duration;

use vexv5_serial::packets::kv::{
    ReadKeyValuePacket, ReadKeyValueReplyPacket, WriteKeyValuePacket, WriteKeyValuePayload,
    WriteKeyValueReplyPacket,
};
use vexv5_serial::string::{FixedLengthString, VarLengthString};

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

    // Set the team number on the brain
    device
        .send_packet(WriteKeyValuePacket::new(WriteKeyValuePayload {
            key: VarLengthString::new("teamnumber".to_string()).unwrap(),
            value: VarLengthString::new(
                "vexide is number 1! vexide is number 1! vexide is number 1! vexide is number 1!"
                    .to_string(),
            )
            .unwrap(),
        }))
        .await
        .unwrap();
    device
        .recieve_packet::<WriteKeyValueReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap();

    // Get the new team number and print it
    device
        .send_packet(ReadKeyValuePacket::new(
            FixedLengthString::new("teamnumber".to_string()).unwrap(),
        ))
        .await
        .unwrap();
    let res = device
        .recieve_packet::<ReadKeyValueReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap()
        .payload
        .try_into_inner()
        .unwrap();

    println!("{:?}", res);
}
