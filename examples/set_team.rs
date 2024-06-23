use std::time::Duration;

use vexv5_serial::connection::serial::find_devices;
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
    let devices = find_devices().unwrap();

    // Open a connection to the device
    let mut connection = devices[0].open(Duration::from_secs(30)).unwrap();

    // Set the team number on the brain
    connection
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
    connection
        .recieve_packet::<WriteKeyValueReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap();

    // Get the new team number and print it
    connection
        .send_packet(ReadKeyValuePacket::new(
            FixedLengthString::new("teamnumber".to_string()).unwrap(),
        ))
        .await
        .unwrap();
    let res = connection
        .recieve_packet::<ReadKeyValueReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap()
        .payload
        .try_into_inner()
        .unwrap();

    println!("{:?}", res);
}
