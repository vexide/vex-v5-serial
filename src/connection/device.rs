//! Implements an async compatible device.

use log::{debug, error, trace, warn};
use std::{pin::Pin, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    select,
    time::{sleep, Instant},
};
use tokio_serial::SerialStream;

use crate::{
    commands::Command, decode::Decode, encode::Encode, packets::decode_header, varint::VarU16,
};

use super::DeviceError;

#[derive(Debug, Clone)]
struct Packet {
    bytes: Vec<u8>,
    used: bool,
    timestamp: Instant,
}

/// The representation of a V5 device that supports async.
pub struct Device {
    system_port: SerialStream,
    user_port: Option<SerialStream>,
    incoming_packets: Vec<Packet>,
}

impl Device {
    pub fn new(system_port: SerialStream, user_port: Option<SerialStream>) -> Self {
        Device {
            system_port,
            user_port,
            incoming_packets: Vec::new(),
        }
    }

    pub async fn execute_command<C: Command>(
        &mut self,
        mut command: C,
    ) -> Result<C::Output, DeviceError> {
        command.execute(self).await
    }

    /// Sends a packet
    pub async fn send_packet(&mut self, packet: impl Encode) -> Result<(), DeviceError> {
        // Encode the packet
        let encoded = packet.encode()?;

        trace!("Sending packet: {:x?}", encoded);

        // Write the packet to the serial port
        match self.system_port.write_all(&encoded).await {
            Ok(_) => (),
            Err(e) => return Err(DeviceError::IoError(e)),
        };

        match self.system_port.flush().await {
            Ok(_) => (),
            Err(e) => return Err(DeviceError::IoError(e)),
        };

        Ok(())
    }

    async fn receive_one_packet(&mut self) -> Result<(), DeviceError> {
        // Read the header into an array
        let mut header = [0u8; 2];
        self.system_port.read_exact(&mut header).await?;

        // Verify that the header is valid
        if let Err(e) = decode_header(header) {
            warn!(
                "Skipping packet with invalid header: {:x?}. Error: {}",
                header, e
            );
            return Ok(());
        }

        // Create a buffer to store the entire packet
        let mut packet = Vec::from(header);

        // Push the command's ID
        packet.push(self.system_port.read_u8().await?);

        // Get the size of the packet
        // We do some extra logic to make sure we only read the necessary amount of bytes
        let first_size_byte = self.system_port.read_u8().await?;
        let size = if VarU16::check_wide(first_size_byte) {
            let second_size_byte = self.system_port.read_u8().await?;
            packet.extend([first_size_byte, second_size_byte]);

            // Decode the size of the packet
            VarU16::decode(vec![first_size_byte, second_size_byte])?
        } else {
            packet.push(first_size_byte);

            // Decode the size of the packet
            VarU16::decode(vec![first_size_byte])?
        }
        .into_inner() as usize;

        // Read the rest of the packet
        let mut payload = vec![0; size];
        self.system_port.read_exact(&mut payload).await?;

        // Completely fill the packet
        packet.extend(payload);

        debug!("Recieved packet: {:x?}", packet);

        // Push the packet to the incoming packets buffer
        self.incoming_packets.push(Packet {
            bytes: packet,
            used: false,
            timestamp: Instant::now(),
        });

        Ok(())
    }

    fn trim_packets(&mut self) {
        debug!(
            "Trimming packets. Length before: {}",
            self.incoming_packets.len()
        );

        // Remove packets that have been used
        self.incoming_packets.retain(|packet| !packet.used);

        // Remove packets that are too old
        self.incoming_packets
            .retain(|packet| packet.timestamp.elapsed() < Duration::from_millis(500));

        debug!(
            "Trimmed packets. Length after: {}",
            self.incoming_packets.len()
        );
    }

    pub async fn recieve_packet<P: Decode>(&mut self, timeout: Duration) -> Result<P, DeviceError> {
        // Return an error if the right packet is not recieved within the timeout
        select! {
            result = async {
                loop {
                    for packet in self.incoming_packets.iter_mut() {
                        if let Ok(decoded) = P::decode(packet.clone().bytes) {
                            packet.used = true;
                            self.trim_packets();
                            return Ok(decoded);
                        }
                    }
                    self.trim_packets();
                    self.receive_one_packet().await?;
                }
            } => result,
            _ = sleep(timeout) => Err(DeviceError::Timeout)
        }
    }

    /// Sends a packet and waits for a response.
    /// This function will retry the handshake `retries` times
    /// before giving up and erroring with the error thrown on the last retry.
    /// # Note
    /// This function will fail immediately if the given packet fails to encode.
    pub async fn packet_handshake<D: Decode>(
        &mut self,
        timeout: Duration,
        retries: usize,
        packet: impl Encode + Clone,
    ) -> Result<D, DeviceError> {
        let mut last_error = DeviceError::Timeout;

        for _ in 0..retries {
            self.send_packet(packet.clone()).await?;
            match self.recieve_packet::<D>(timeout).await {
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

impl AsyncRead for Device {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        // If the user port is available, then just read from it
        if let Some(ref mut p) = self.user_port {
            AsyncRead::poll_read(Pin::new(p), cx, buf)
        } else {
            // If not, then error
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                DeviceError::NoWriteOnWireless,
            )))
        }
    }
}

impl AsyncWrite for Device {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        if let Some(ref mut p) = self.user_port {
            AsyncWrite::poll_write(Pin::new(p), cx, buf)
        } else {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                DeviceError::NoWriteOnWireless,
            )))
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        if let Some(ref mut p) = self.user_port {
            AsyncWrite::poll_flush(Pin::new(p), cx)
        } else {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                DeviceError::NoWriteOnWireless,
            )))
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        if let Some(ref mut p) = self.user_port {
            AsyncWrite::poll_shutdown(Pin::new(p), cx)
        } else {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                DeviceError::NoWriteOnWireless,
            )))
        }
    }
}
