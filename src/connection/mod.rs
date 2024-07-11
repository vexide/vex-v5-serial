//! Implements functions and structures for interacting with vex devices.

use std::{future::Future, time::Instant};

use log::{debug, error, warn};
use std::time::Duration;
use thiserror::Error;

use crate::{
    commands::Command,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    packets::cdc2::Cdc2Ack,
};

pub mod bluetooth;
pub mod generic;
pub mod serial;

#[derive(Debug, Clone)]
pub(crate) struct RawPacket {
    bytes: Vec<u8>,
    used: bool,
    timestamp: Instant,
}
impl RawPacket {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            used: false,
            timestamp: Instant::now(),
        }
    }

    pub fn is_obsolete(&self, timeout: Duration) -> bool {
        self.timestamp.elapsed() > timeout || self.used
    }

    /// Decodes the packet into the given type.
    /// If successful, marks the packet as used.
    /// # Note
    /// This function will **NOT** fail if the packet has already been used.
    pub fn decode_and_use<D: Decode>(&mut self) -> Result<D, DecodeError> {
        let decoded = D::decode(self.bytes.clone())?;
        self.used = true;
        Ok(decoded)
    }
}
/// Removes old and used packets from the incoming packets buffer.
pub(crate) fn trim_packets(packets: &mut Vec<RawPacket>) {
    debug!("Trimming packets. Length before: {}", packets.len());

    // Remove packets that are obsolete
    packets.retain(|packet| !packet.is_obsolete(Duration::from_secs(2)));

    debug!("Trimmed packets. Length after: {}", packets.len());
}

/// Represents an open connection to a V5 peripheral.
#[allow(async_fn_in_trait)]
pub trait Connection {
    fn connection_type(&self) -> ConnectionType;

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

    /// Read user program output.
    fn read_user(
        &mut self,
        buf: &mut [u8],
    ) -> impl Future<Output = Result<usize, ConnectionError>>;

    /// Write to user program stdio.
    fn write_user(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, ConnectionError>>;

    /// Executes a [`Command`].
    async fn execute_command<C: Command>(
        &mut self,
        mut command: C,
    ) -> Result<C::Output, ConnectionError> {
        command.execute(self).await
    }

    /// Sends a packet and waits for a response.
    ///
    /// This function will retry the handshake `retries` times
    /// before giving up and erroring with the error thrown on the last retry.
    ///
    /// # Note
    ///
    /// This function will fail immediately if the given packet fails to encode.
    async fn packet_handshake<D: Decode>(
        &mut self,
        timeout: Duration,
        retries: usize,
        packet: impl Encode + Clone,
    ) -> Result<D, ConnectionError> {
        let mut last_error = ConnectionError::Timeout;

        for _ in 0..retries {
            self.send_packet(packet.clone()).await?;
            match self.receive_packet::<D>(timeout).await {
                Ok(decoded) => return Ok(decoded),
                Err(e) => {
                    warn!("Handshake failed: {}. Retrying...", e);
                    last_error = e;
                }
            }
        }
        error!(
            "Handshake failed after {} retries with error: {}",
            retries, last_error
        );
        Err(last_error)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConnectionType {
    Wired,
    Controller,
    Bluetooth,
}
impl ConnectionType {
    pub fn is_wired(&self) -> bool {
        matches!(self, ConnectionType::Wired)
    }
    pub fn is_controller(&self) -> bool {
        matches!(self, ConnectionType::Controller)
    }
    pub fn is_bluetooth(&self) -> bool {
        matches!(self, ConnectionType::Bluetooth)
    }
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
    BluetoothError(#[from] btleplug::Error),
    #[error("The device is not a supported vex device")]
    InvalidDevice,
    #[error("Invalid Magic Number")]
    InvalidMagic,
    #[error("Not connected to the device")]
    NotConnected,
    #[error("No Bluetooth Adapter Found")]
    NoBluetoothAdapter,
    #[error("Expected a Bluetooth characteristic that didn't exist")]
    MissingCharacteristic,
    #[error("Authentication PIN code was incorrect")]
    IncorrectPin,
    #[error("Pairing is required")]
    PairingRequired,
    #[error("Pairing is not supported over any connection other than Bluetooth")]
    PairingNotSupported,
}
