#[tokio::main]
async fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    // Set the team number on the brain
    device
        .send_packet_request(vexv5_serial::protocol::KVWrite("teamnumber", "3636"))
        .await
        .unwrap();

    // Get the new team number and print it
    let res = device
        .send_packet_request(vexv5_serial::protocol::KVRead("teamnumber"))
        .await
        .unwrap();

    println!("{}", res);
}
