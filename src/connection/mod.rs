//! Implements functions and structures for interacting with vex devices.

use std::future::Future;

use std::time::Duration;
use thiserror::Error;

use crate::{
    commands::Command,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    packets::cdc2::Cdc2Ack,
};

pub mod bluetooth;
pub mod serial;

/// Represents an open connection to a V5 peripheral.
#[allow(async_fn_in_trait)]
pub trait Connection: Sized {
    /// Executes a [`Command`].
    async fn execute_command<C: Command>(
        &mut self,
        mut command: C,
    ) -> Result<C::Output, ConnectionError> {
        command.execute(self).await
    }

    /// Sends a packet.
    fn send_packet(
        &mut self,
        packet: impl Encode,
    ) -> impl Future<Output = Result<(), ConnectionError>>;

    /// Receives a packet.
    fn receive_packet<P: Decode>(
        &mut self,
        timeout: Duration,
    ) -> impl Future<Output = Result<P, ConnectionError>>;

    /// Sends a packet and waits for a response.
    ///
    /// This function will retry the handshake `retries` times
    /// before giving up and erroring with the error thrown on the last retry.
    ///
    /// # Note
    ///
    /// This function will fail immediately if the given packet fails to encode.
    fn packet_handshake<D: Decode>(
        &mut self,
        timeout: Duration,
        retries: usize,
        packet: impl Encode + Clone,
    ) -> impl Future<Output = Result<D, ConnectionError>>;
}

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
    #[error("NACK received: {0:?}")]
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
