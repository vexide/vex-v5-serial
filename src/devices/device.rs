//! Implements an async compatible device.

use std::{pin::Pin, time::Duration};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    select,
    time::{sleep_until, Instant},
};
use tokio_serial::SerialStream;

use crate::{
    commands::Command,
    packets::{cdc2::Cdc2Ack, decode_header, Decode, DecodeError, Encode, EncodeError, VarU16},
};

#[derive(Error, Debug)]
pub enum DeviceError {
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
}

/// The representation of a V5 device that supports async.
pub struct Device {
    system_port: SerialStream,
    user_port: Option<SerialStream>,
    read_buffer: Vec<u8>,
    user_read_size: u8,
}

impl Device {
    pub fn new(system_port: SerialStream, user_port: Option<SerialStream>) -> Self {
        Device {
            system_port,
            user_port,
            read_buffer: Vec::new(),
            user_read_size: 0x20, // By default, read chunks of 32 bytes
        }
    }

    /// Updates the size of the chunks to read from the system port when a user port is not available
    pub fn update_user_read_size(&mut self, user_read_size: u8) {
        self.user_read_size = user_read_size;
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

        println!("Sending packet: {:x?}", encoded);

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

    pub async fn recieve_packet<P: Decode>(&mut self, timeout: Duration) -> Result<P, DeviceError> {
        let time = Instant::now();
        let mut header = [0; 2];

        // Return an error if the header is not recieved within the timeout
        select! {
            result = self.system_port.read_exact(&mut header) =>
                match result {
                    Ok(_) => Ok(()),
                    Err(e) => Err(DeviceError::IoError(e)),
                },
            _ = sleep_until(time + timeout) => Err(DeviceError::Timeout)
        }?;
        // Verify that the header is correct
        decode_header(header)?;

        // Start to accumulate header/metadata bits
        let mut packet = Vec::from(header);
        // Add the command id
        packet.push(self.system_port.read_u8().await?);

        // Get the length of the packet payload
        let first_size_byte = self.system_port.read_u8().await?;
        let size = if VarU16::check_wide(first_size_byte) {
            println!("Wide size byte");
            let second_size_byte = self.system_port.read_u8().await?;
            packet.extend([first_size_byte, second_size_byte]);
            VarU16::decode(vec![first_size_byte, second_size_byte])?
        } else {
            packet.push(first_size_byte);
            VarU16::decode(vec![first_size_byte])?
        }
        .into_inner() as usize;

        // Read the rest of the packet
        let mut payload = vec![0; size];
        self.system_port.read_exact(&mut payload).await?;
        packet.extend(payload);
        println!("Recieved packet: {:x?}", packet);

        // Decode the packet
        P::decode(packet).map_err(Into::into)
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
                crate::errors::DeviceError::NoWriteOnWireless,
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
                crate::errors::DeviceError::NoWriteOnWireless,
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
                crate::errors::DeviceError::NoWriteOnWireless,
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
                crate::errors::DeviceError::NoWriteOnWireless,
            )))
        }
    }
}
