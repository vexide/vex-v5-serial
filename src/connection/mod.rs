//! Implements functions and structures for interacting with vex devices.

use std::{future::Future, time::Instant};

use log::{error, trace, warn};
use std::time::Duration;

use crate::{
    commands::Command,
    decode::{Decode, DecodeError},
    encode::Encode,
    packets::cdc2::Cdc2Ack, string::FixedStringSizeError,
};

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(all(feature = "serial", feature = "bluetooth"))]
pub mod generic;
#[cfg(feature = "serial")]
pub mod serial;

pub trait CheckHeader {
    fn has_valid_header(data: &[u8]) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn check_header<H: CheckHeader>(&self) -> bool {
        H::has_valid_header(&self.bytes)
    }

    /// Decodes the packet into the given type.
    /// If successful, marks the packet as used.
    /// # Note
    /// This function will **NOT** fail if the packet has already been used.
    pub fn decode_and_use<D: Decode>(&mut self) -> Result<D, DecodeError> {
        let decoded = D::decode(&mut self.bytes.as_slice())?;
        self.used = true;
        Ok(decoded)
    }
}
/// Removes old and used packets from the incoming packets buffer.
pub(crate) fn trim_packets(packets: &mut Vec<RawPacket>) {
    trace!("Trimming packets. Length before: {}", packets.len());

    // Remove packets that are obsolete
    packets.retain(|packet| !packet.is_obsolete(Duration::from_secs(2)));

    trace!("Trimmed packets. Length after: {}", packets.len());
}

/// Represents an open connection to a V5 peripheral.
#[allow(async_fn_in_trait)]
pub trait Connection {
    type Error: std::error::Error + From<DecodeError> + From<Cdc2Ack> + From<FixedStringSizeError>;

    fn connection_type(&self) -> ConnectionType;

    /// Sends a packet.
    fn send(&mut self, packet: impl Encode) -> impl Future<Output = Result<(), Self::Error>>;

    /// Receives a packet.
    fn recv<P: Decode + CheckHeader>(
        &mut self,
        timeout: Duration,
    ) -> impl Future<Output = Result<P, Self::Error>>;

    /// Read user program output.
    fn read_user(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Write to user program stdio.
    fn write_user(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Executes a [`Command`].
    fn execute_command<C: Command>(
        &mut self,
        command: C,
    ) -> impl Future<Output = Result<C::Output, Self::Error>> {
        command.execute(self)
    }

    /// Sends a packet and waits for a response.
    ///
    /// This function will retry the handshake `retries` times
    /// before giving up and erroring with the error thrown on the last retry.
    ///
    /// # Note
    ///
    /// This function will fail immediately if the given packet fails to encode.
    async fn handshake<D: Decode + CheckHeader>(
        &mut self,
        timeout: Duration,
        retries: usize,
        packet: impl Encode + Clone,
    ) -> Result<D, Self::Error> {
        let mut last_error = None;

        for _ in 0..=retries {
            self.send(packet.clone()).await?;
            match self.recv::<D>(timeout).await {
                Ok(decoded) => return Ok(decoded),
                Err(e) => {
                    warn!(
                        "Handshake failed while waiting for {}: {:?}. Retrying...",
                        std::any::type_name::<D>(),
                        e
                    );
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
