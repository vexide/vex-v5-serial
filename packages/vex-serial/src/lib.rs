//! Crate for communicating with the VEX Robotics products.
//! 
//! Not affiliated with Innovation First Inc.

pub use vex_cdc as protocol;
#[cfg(any(feature = "serial-blocking", feature = "serial-async", feature = "bluetooth"))]
pub(crate) mod packet;
mod connection;

pub use connection::{Connection, AsyncConnection, ConnectionError, ConnectionType};

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(any(feature = "serial-blocking", feature = "serial-async"))]
pub mod serial;
