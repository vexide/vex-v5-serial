//! Implements functions and structures for interacting with vex devices.

use thiserror::Error;

use crate::{decode::DecodeError, encode::EncodeError, packets::cdc2::Cdc2Ack};

pub mod bluetooth;
pub mod serial;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Packet encoding error: {0}")]
    EncodeError(#[from] EncodeError),
    #[error("Packet decoding error: {0}")]
    DecodeError(#[from] DecodeError),
    #[error("Packet timeout")]
    Timeout,
    #[error("NACK recieved: {0:?}")]
    Nack(Cdc2Ack),
    #[error("Serialport Error")]
    SerialportError(#[from] tokio_serial::Error),
    #[error("The user port can not be written to over wireless")]
    NoWriteOnWireless,
    #[error("Bluetooth Error")]
    BluetoothError(#[from] bluest::Error),
    #[error("The device is not a supported vex device")]
    InvalidDevice,
    #[error("Invalid Magic Number")]
    InvalidMagic,
    #[error("Not connected to the device")]
    NotConnected,
    #[error("No Bluetooth Adapter Found")]
    NoBluetoothAdapter,
}