//! Implementation of the VEX Robotics CDC protocol in Rust.
//!
//! This crate allows you to encode and decode packets used to communicate
//! with products sold by [VEX Robotics] using their CDC (**C**ommunications
//! **D**evice **C**lass) protocol. The protocol can be used to upload programs
//! and interact with VEX brains and other hardware over USB and bluetooth.
//!
//! Currently, most packets supported by the [V5 Brain] and [V5 Controller] are
//! implemented, though the packets provided by this crate are non-exhaustive.
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

//#![no_std]

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
pub use decode::{Decode, DecodeError, DecodeErrorKind, DecodeWithLength};
pub use encode::Encode;
pub use string::{FixedString, FixedStringSizeError};
pub use varint::{VarU16, VarU16SizeError};
pub use version::Version;

macro_rules! cdc2_pair {
    ($command_type:ty => $reply_type:ty, $cmd:expr, $ecmd:expr$(,)?) => {
        impl crate::cdc::CdcCommand for $command_type {
            const CMD: u8 = $cmd;
            type Reply = Result<$reply_type, crate::cdc2::Cdc2Ack>;
        }
        impl crate::cdc2::Cdc2Command for $command_type {
            const ECMD: u8 = $ecmd;
        }

        impl crate::decode::Decode for Result<$reply_type, crate::cdc2::Cdc2Ack> {
            fn decode(data: &mut &[u8]) -> Result<Self, crate::decode::DecodeError> {
                crate::cdc2::decode_cdc2_reply::<Self, $reply_type>(data)
            }
        }

        impl crate::cdc::CdcReply for Result<$reply_type, crate::cdc2::Cdc2Ack> {
            const CMD: u8 = $cmd;
            type Command = $command_type;
        }
        impl crate::cdc2::Cdc2Reply for Result<$reply_type, crate::cdc2::Cdc2Ack> {
            const ECMD: u8 = $ecmd;
        }
    };
}
pub(crate) use cdc2_pair;
