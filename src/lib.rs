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
#![feature(iter_next_chunk)]

pub mod checks;
pub mod commands;
pub mod devices;
pub mod errors;
pub mod packets;
pub mod protocol;
pub mod v5;

use crc::Algorithm;

/// Vex uses CRC16/XMODEM as the CRC16.
pub const VEX_CRC16: Algorithm<u16> = crc::CRC_16_XMODEM;

/// Vex uses a CRC32 that I found on page 6 of this document:
/// <https://www.matec-conferences.org/articles/matecconf/pdf/2016/11/matecconf_tomsk2016_04001.pdf>
/// I literally just discovered it by guessing and checking against the PROS implementation.
pub const VEX_CRC32: Algorithm<u32> = Algorithm {
    poly: 0x04C11DB7,
    init: 0x00000000,
    refin: false,
    refout: false,
    xorout: 0x00000000,
    check: 0x89A1897F,
    residue: 0x00000000,
    width: 32,
};
