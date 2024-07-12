//! Implements functions and structures for interacting with vex devices.

use std::{future::Future, time::Instant};

use log::{debug, error, warn};
use std::time::Duration;

use crate::{
    commands::Command,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    packets::cdc2::Cdc2Ack,
};

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(feature = "bluetooth")]
pub mod generic;
#[cfg(feature = "serial")]
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
    type Error: std::error::Error + From<EncodeError> + From<DecodeError> + From<Cdc2Ack>;

    fn connection_type(&self) -> ConnectionType;

    /// Sends a packet.
    fn send_packet(&mut self, packet: impl Encode)
        -> impl Future<Output = Result<(), Self::Error>>;

    /// Receives a packet.
    fn receive_packet<P: Decode>(
        &mut self,
        timeout: Duration,
    ) -> impl Future<Output = Result<P, Self::Error>>;

    /// Read user program output.
    fn read_user(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Write to user program stdio.
    fn write_user(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Executes a [`Command`].
    async fn execute_command<C: Command>(
        &mut self,
        mut command: C,
    ) -> Result<C::Output, Self::Error> {
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
    ) -> Result<D, Self::Error> {
        let mut last_error = None;

        for _ in 0..retries {
            self.send_packet(packet.clone()).await?;
            match self.receive_packet::<D>(timeout).await {
                Ok(decoded) => return Ok(decoded),
                Err(e) => {
                    warn!("Handshake failed: {:?}. Retrying...", e);
                    last_error = Some(e);
                }
            }
        }
        error!(
            "Handshake failed after {} retries with error: {:?}",
            retries, last_error
        );
        Err(last_error.unwrap())
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