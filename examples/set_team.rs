fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    // Set the team number on the brain
    device
        .send_request(vexv5_serial::commands::KVWrite("teamnumber", "3636"))
        .unwrap();

    // Get the new team number and print it
    let res = device
        .send_request(vexv5_serial::commands::KVRead("teamnumber"))
        .unwrap();

    println!("{}", res);
}
