use std::time::Duration;

use vexv5_serial::packets::{
    cdc2::Cdc2CommandPayload,
    kv::{
        ReadKeyValuePacket, ReadKeyValueReplyPacket, WriteKeyValuePacket, WriteKeyValuePayload,
        WriteKeyValueReplyPacket,
    },
    TerminatedFixedLengthString, VarLengthString,
};

#[tokio::main]
async fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    // Set the team number on the brain
    device
        .send_packet(WriteKeyValuePacket::new(Cdc2CommandPayload::new(
            WriteKeyValuePayload {
                key: VarLengthString::new("teamnumber".to_string()).unwrap(),
                value: VarLengthString::new("bob".to_string()).unwrap(),
            },
        )))
        .await
        .unwrap();
    device
        .recieve_packet::<WriteKeyValueReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap();

    // Get the new team number and print it
    device
        .send_packet(ReadKeyValuePacket::new(Cdc2CommandPayload::new(
            TerminatedFixedLengthString::new("teamnumber".to_string()).unwrap(),
        )))
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
