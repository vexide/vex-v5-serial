use std::time::Duration;

use vex_cdc::{
    DecodeError, FixedStringSizeError,
    cdc::{CdcCommand, CdcReply},
    cdc2::Cdc2Ack,
};

/// Represents an open blocking connection to a VEX peripheral.
#[allow(async_fn_in_trait)]
pub trait Connection {
    type Error: ConnectionError;

    fn connection_type(&self) -> ConnectionType;

    /// Sends a packet.
    fn send(&mut self, packet: impl CdcCommand) -> Result<(), Self::Error>;

    /// Receives a packet.
    fn recv<P: CdcReply>(&mut self, timeout: Duration) -> Result<P, Self::Error>;

    /// Sends a packet and waits for a response.
    ///
    /// This function will retry the handshake `retries` times
    /// before giving up and erroring with the error thrown on the last retry.
    ///
    /// # Note
    ///
    /// This function will fail immediately if the given packet fails to encode.
    fn handshake<P: CdcCommand + Clone>(
        &mut self,
        packet: P,
        timeout: Duration,
        retries: usize,
    ) -> Result<P::Reply, Self::Error> {
        let mut attempt = 0;

        loop {
            self.send(packet.clone())?;

            match self.recv::<P::Reply>(timeout) {
                Ok(decoded) => return Ok(decoded),
                Err(e) if e.should_retry() && attempt < retries => {
                    attempt += 1;
                    log::warn!(
                        "Handshake timed out while waiting for {}: {:?}. Retrying ({}/{})...",
                        std::any::type_name::<P::Reply>(),
                        e,
                        attempt,
                        retries
                    );
                }
                Err(e) => {
                    log::error!(
                        "Handshake failed after {} retries while waiting for {}: {:?}",
                        retries,
                        std::any::type_name::<P::Reply>(),
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Read user program output.
    fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Write to user program stdio.
    fn write_user(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
}

/// Represents an open asynchronous connection to a VEX peripheral.
#[allow(async_fn_in_trait)]
pub trait AsyncConnection {
    type Error: ConnectionError;

    fn connection_type(&self) -> ConnectionType;

    /// Sends a packet.
    fn send(&mut self, packet: impl CdcCommand) -> impl Future<Output = Result<(), Self::Error>>;

    /// Receives a packet.
    fn recv<P: CdcReply>(
        &mut self,
        timeout: Duration,
    ) -> impl Future<Output = Result<P, Self::Error>>;

    /// Sends a packet and waits for a response.
    ///
    /// This function will retry the handshake `retries` times
    /// before giving up and erroring with the error thrown on the last retry.
    ///
    /// # Note
    ///
    /// This function will fail immediately if the given packet fails to encode.
    async fn handshake<P: CdcCommand + Clone>(
        &mut self,
        packet: P,
        timeout: Duration,
        retries: usize,
    ) -> Result<P::Reply, Self::Error> {
        let mut attempt = 0;

        loop {
            self.send(packet.clone()).await?;

            match self.recv::<P::Reply>(timeout).await {
                Ok(decoded) => return Ok(decoded),
                Err(e) if e.should_retry() && attempt < retries => {
                    attempt += 1;
                    log::warn!(
                        "Handshake timed out while waiting for {}: {:?}. Retrying ({}/{})...",
                        std::any::type_name::<P::Reply>(),
                        e,
                        attempt,
                        retries
                    );
                }
                Err(e) => {
                    log::error!(
                        "Handshake failed after {} retries while waiting for {}: {:?}",
                        retries,
                        std::any::type_name::<P::Reply>(),
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Read user program output.
    fn read_user(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    /// Write to user program stdio.
    fn write_user(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, Self::Error>>;
}

/// An error type returned by something implementing [`Connection`].
pub trait ConnectionError:
    std::error::Error + From<DecodeError> + From<Cdc2Ack> + From<FixedStringSizeError>
{
    fn should_retry(&self) -> bool;
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

    pub fn max_chunk_size(&self, window_size: u16) -> u16 {
        const USER_PROGRAM_CHUNK_SIZE: u16 = 4096;

        #[cfg(feature = "bluetooth")]
        {
            use crate::bluetooth::BluetoothConnection;

            if self.is_bluetooth() {
                let max_chunk_size =
                    (BluetoothConnection::MAX_PACKET_SIZE as u16).min(window_size / 2) - 14;
                max_chunk_size - (max_chunk_size % 4)
            } else if window_size > 0 && window_size <= USER_PROGRAM_CHUNK_SIZE {
                window_size
            } else {
                USER_PROGRAM_CHUNK_SIZE
            }
        }

        #[cfg(not(feature = "bluetooth"))]
        if window_size > 0 && window_size <= USER_PROGRAM_CHUNK_SIZE {
            window_size
        } else {
            USER_PROGRAM_CHUNK_SIZE
        }
    }
}
