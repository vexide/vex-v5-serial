//! Implementation of the VEX Robotics CDC protocol in Rust.
//! 
//! This crate allows you to encode and decode packets used to communicate
//! with products sold by [VEX Robotics] using their CDC (**C**ommunications
//! **D**evice **C**lass) protocol. The protocol can be used to upload programs
//! and interact with VEX brains and other hardware over USB and bluetooth.
//! 
//! Currently, most packets supported by the [V5 Brain] and [V5 Controller] are
//! implemented, though the packets provided by this crate are not exhaustive.
//! 
//! [VEX Robotics]: https://www.vexrobotics.com/
//! [V5 Brain]: https://www.vexrobotics.com/276-4810.html
//! [V5 Controller]: https://www.vexrobotics.com/276-4820.html
//! 
//! This crate is used as a backing implementation for vexide's [vex-v5-serial]
//! library and [cargo-v5].
//! 
//! [vex-v5-serial]: http://crates.io/crates/vex-v5-serial
//! [cargo-v5]: https://github.com/vexide/cargo-v5

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

/// Starting byte sequence for all device-bound CDC packets.
pub const COMMAND_HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

/// Starting byte sequence used for all host-bound CDC packets.
pub const REPLY_HEADER: [u8; 2] = [0xAA, 0x55];
