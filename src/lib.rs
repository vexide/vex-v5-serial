//! Crate for interacting with the Vex V5 Robot brain. Not affiliated with Innovation First Inc.
//!
//! This crate is structured around two key traits: [`Encode`](encode::Encode) and [`Decode`](decode::Decode).
//! These traits are used to encode messages to be sent to the brain and decode messages received from the brain.
//! All packet types in this library have either an [`Encode`](encode::Encode) or [`Decode`](decode::Decode) implementation.
//!
//! Because manually sending and receiving packets is a chore, this library also provides high level [`Command`](commands::Command)s.
//! These commands provide easier ways to perform complicated tasks, such as uploading a program.


//TODO: Figure out a better alternate to this feature
#![feature(array_try_from_fn)]

pub mod array;
pub mod choice;
pub mod commands;
pub mod connection;
pub mod crc;
pub mod decode;
pub mod encode;
pub mod packets;
pub mod string;
pub mod timestamp;
pub mod varint;
pub mod version;
