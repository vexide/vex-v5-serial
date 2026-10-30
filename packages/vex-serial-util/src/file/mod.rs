use std::{
    io::Write, time::{SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(feature = "async")]
pub use asynchronous::*;
use flate2::{Compression, GzBuilder};
use vex_cdc::{FixedString, cdc2::file::FileVendor};

#[cfg(feature = "blocking")]
pub mod blocking;

/// The epoch of the serial protocol's timestamps.
pub const J2000_EPOCH: u64 = 946684800;

pub fn j2000_timestamp() -> i32 {
    let unix_timestamp_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    (unix_timestamp_secs - J2000_EPOCH) as i32
}

pub const PROS_HOT_BIN_LOAD_ADDR: u32 = 0x7800000;
pub const USER_PROGRAM_LOAD_ADDR: u32 = 0x3800000;

pub struct LinkedFile {
    pub file_name: FixedString<23>,
    pub vendor: FileVendor,
}

#[derive(Debug)]
pub enum ProgramData {
    Monolith(Vec<u8>),
    HotCold {
        hot: Option<Vec<u8>>,
        cold: Option<Vec<u8>>,
    },
}

/// Apply gzip compression to the given data
pub(crate) fn compress_data(data: &mut Vec<u8>) {
    let mut encoder = GzBuilder::new().write(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    *data = encoder.finish().unwrap();
}
