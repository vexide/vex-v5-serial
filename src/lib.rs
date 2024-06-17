//! Crate for interacting with the Vex V5 Robot brain. Not affiliated with Innovation First Inc.
//!
//! This crate is structured so that each "command" that can be sent to the robot brain has it's own structure associated with it.
//! Each "command" also has it's own response associated with it. Commands are implemented using the `Command` trait,
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

pub mod checks;
pub mod devices;
pub mod errors;
pub mod protocol;
pub mod v5;

use crc::Algorithm;

pub mod extended {
    pub use crate::protocol::{Extended, ExtendedResponse};
}

pub mod kv {
    pub use crate::protocol::{KVRead, KVWrite};
}

pub mod system {
    pub use crate::protocol::{GetSystemVersion, V5SystemVersion};

    pub use crate::v5::{V5BrainFlags, V5ControllerFlags, VexProductType};
}

pub mod remote {
    pub use crate::protocol::SwitchChannel;

    pub use crate::v5::V5ControllerChannel;
}
/// Structs in this crate will be used a lot, so FileTransfer is shortened to FT
pub mod file {
    pub use crate::protocol::{
        FileTransferExit as FTExit, FileTransferInit as FTInit,
        FileTransferInitResponse as FTInitResponse, FileTransferRead as FTRead,
        FileTransferSetLink as FTSetLink, FileTransferWrite as FTWrite, GetFileMetadataByName,
    };

    pub use crate::v5::{
        FileMetadataByName, FileTransferComplete as FTComplete, FileTransferFunction as FTFunction,
        FileTransferOptions as FTOptions, FileTransferTarget as FTTarget,
        FileTransferType as FTType, FileTransferVID as FTVID,
    };
}

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
