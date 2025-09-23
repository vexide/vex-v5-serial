//! Implementation of the VEX Robotics CDC communication protocol in Rust.

#![no_std]

extern crate alloc;

pub mod cdc;
pub mod cdc2;

mod crc;
mod decode;
mod encode;
mod string;
mod varint;
mod version;

pub use crc::{VEX_CRC16, VEX_CRC32};
pub use decode::{Decode, DecodeError, DecodeWithLength};
pub use encode::{Encode, MessageEncoder};
pub use string::{FixedString, FixedStringSizeError};
pub use varint::{VarU16, VarU16SizeError};
pub use version::Version;

/// Starting byte sequence for all device-bound packets.
pub const COMMAND_HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

/// Starting byte sequence used for all host-bound packets.
pub const REPLY_HEADER: [u8; 2] = [0xAA, 0x55];
