//! Crate for interacting with the Vex V5 Robot brain. Not affiliated with Innovation First Inc.
//!
//! This crate is structured so that each "packet" that can be sent to the robot brain has it's own structure associated with it.
//! Each "packet" also has it's own response associated with it. Packets are implemented using the `Packet` trait,
//! which currently provides a function to encode the implementing structure to a `Vec<u8>` and a function to decode from a Read stream to the implementing structure.
//!
//! V5 devices do not have to be accessed over a serial port, but helper functions are provided for finding and opening serial ports.
//! Please note that this example may panic and if it succeeds it *will* change the team number on your brain
//! ```rust
//!
//! // Find all vex devices on the serial ports
//! let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices()?;
//!
//! // Open the device
//! let mut device = vex_ports[0].open()?;
//!
//! // Set the team number on the brain
//! let _ = device.send_request(vexv5_serial::commands::KVWrite("teamnumber", "ABCD")).unwrap();
//!
//! // Get the new team number and print it
//! let res = device.send_request(vexv5_serial::commands::KVRead("teamnumber")).unwrap();
//!
//! println!("{}", res);
//!
//! ```

//TODO: Figure out a better alternate to this feature
#![feature(array_try_from_fn)]

pub mod array;
pub mod choice;
pub mod commands;
pub mod crc;
pub mod decode;
pub mod connection;
pub mod encode;
pub mod errors;
pub mod packets;
pub mod string;
pub mod timestamp;
pub mod varint;
pub mod version;
