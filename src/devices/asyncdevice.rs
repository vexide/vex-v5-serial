//! Implements an async compatible device.

use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The representation of a V5 device that supports async.
pub struct AsyncDevice<S: AsyncReadExt + AsyncWriteExt, U: AsyncReadExt + AsyncWriteExt> {
    system_port: S,
    user_port: Option<U>,
    read_buffer: Vec<u8>,
    user_read_size: u8,
}

impl<S: AsyncReadExt + AsyncWriteExt + Unpin, U: AsyncReadExt + AsyncWriteExt + Unpin>
    AsyncDevice<S, U>
{
    pub fn new(system_port: S, user_port: Option<U>) -> Self {
        AsyncDevice {
            system_port,
            user_port,
            read_buffer: Vec::new(),
            user_read_size: 0x20, // By default, read chunks of 32 bytes
        }
    }

    /// Returns true if this device is a controller
    pub async fn is_controller(&mut self) -> Result<bool, crate::errors::DecodeError> {
        // Get the vex system info
        // Return true if this is a controller
        Ok(
            match self
                .send_request(crate::system::GetSystemVersion())
                .await?
                .product_type
            {
                crate::system::VexProductType::V5Brain(_) => false,
                crate::system::VexProductType::V5Controller(_) => true,
            },
        )
    }

    /// Updates the size of the chunks to read from the system port when a user port is not available
    pub fn update_user_read_size(&mut self, user_read_size: u8) {
        self.user_read_size = user_read_size;
    }

    /// Sends a command and recieves its response
    pub async fn send_request<C: crate::commands::Command + Copy>(
        &mut self,
        command: C,
    ) -> Result<C::Response, crate::errors::DecodeError> {
        // Send the command over the system port
        self.send_command(command).await?;

        // Wait for the response
        self.response_for::<C>(std::time::Duration::new(
            crate::devices::SERIAL_TIMEOUT_SECONDS,
            crate::devices::SERIAL_TIMEOUT_NS,
        ))
        .await
    }

    /// Sends a command
    pub async fn send_command<C: crate::commands::Command + Copy>(
        &mut self,
        command: C,
    ) -> Result<(), crate::errors::DecodeError> {
        // Encode the command
        let encoded = command.encode_request()?;

        // Create the packet
        let packet = if encoded.0 == 0x56 {
            // If it is an extended packet, just pass the data along
            encoded.1
        } else {
            // If not, then create the simple packet
            let mut data = vec![0xc9, 0x36, 0xb8, 0x47, encoded.0];
            data.extend(encoded.1);
            data
        };

        // Write the command to the serial port
        match self.system_port.write_all(&packet).await {
            Ok(_) => (),
            Err(e) => return Err(crate::errors::DecodeError::IoError(e)),
        };

        match self.system_port.flush().await {
            Ok(_) => (),
            Err(e) => return Err(crate::errors::DecodeError::IoError(e)),
        };

        Ok(())
    }

    /// Recieves a response for a command
    pub async fn response_for<C: crate::commands::Command + Copy>(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<C::Response, crate::errors::DecodeError> {
        // We need to wait to recieve the header of a packet.
        // The header should be the bytes [0xAA, 0x55]

        // This header needs to be recieved within the timeout.
        // If it is not recieved within the timeout, then we need to return an error.
        // Begin the countdown now:
        let countdown = std::time::SystemTime::now() + timeout;

        // Create a buffer for the header bytes
        // This is configurable just in case vex changes the header bytes on us.
        let expected_header: [u8; 2] = [0xAA, 0x55];
        let mut header_index = 0; // This represents what index in the header we will be checking next.

        // The way this works is we recieve a byte from the device.
        // If it matches the current byte (expected_header[header_index]), then we increment the header_index.
        // If the header_index is equal to the length of the header, then we know we have recieved the header.
        // If the header_index is not equal to the length of the header, then we need to keep recieving bytes until we have recieved the header.
        // If an unexpected byte is recieved, reset header_index to zero.
        while header_index < expected_header.len() {
            // If the timeout has elapsed, then we need to return an error.
            // We need to do this first just in case we actually do recieve the header
            // before the timeout has elapsed.
            if countdown < std::time::SystemTime::now() {
                return Err(crate::errors::DecodeError::HeaderTimeout);
            }

            // Recieve a single bytes
            let mut b: [u8; 1] = [0];
            match self.system_port.read_exact(&mut b).await {
                // Do some match magic to convert the error types
                Ok(v) => Ok(v),
                Err(e) => Err(crate::errors::DecodeError::IoError(e)),
            }?;
            let b = b[0];

            if b == expected_header[header_index] {
                header_index += 1;
            } else {
                header_index = 0;
            }
        }

        // Now that we know we have recieved the header, we need to recieve the rest of the packet.

        // First create a vector containing the entirety of the recieved packet
        let mut packet: Vec<u8> = Vec::from(expected_header);

        // Read int he next two bytes
        let mut b: [u8; 2] = [0; 2];
        match self.system_port.read_exact(&mut b).await {
            // Do some match magic to convert the error types
            Ok(v) => Ok(v),
            Err(e) => Err(crate::errors::DecodeError::IoError(e)),
        }?;
        packet.extend_from_slice(&b);

        // Get the command byte and the length byte of the packet
        let command = b[0];

        // We may need to modify the length of the packet if it is an extended command
        // Extended commands use a u16 instead of a u8 for the length.
        let length = if 0x56 == command && b[1] & 0x80 == 0x80 {
            // Read the lower bytes
            let mut bl: [u8; 1] = [0];
            match self.system_port.read_exact(&mut bl).await {
                // Do some match magic to convert the error types
                Ok(v) => Ok(v),
                Err(e) => Err(crate::errors::DecodeError::IoError(e)),
            }?;
            packet.push(bl[0]);

            (((b[1] & 0x7f) as u16) << 8) | (bl[0] as u16)
        } else {
            b[1] as u16
        };

        // Read the rest of the payload
        let mut payload: Vec<u8> = vec![0; length as usize];
        // DO NOT CHANGE THIS TO READ. read_exact is required to suppress
        // CRC errors and missing data.
        match self.system_port.read_exact(&mut payload).await {
            // Do some match magic to convert the error types
            Ok(v) => Ok(v),
            Err(e) => Err(crate::errors::DecodeError::IoError(e)),
        }?;
        packet.extend(&payload);

        C::decode_response(command, payload)
    }

    /// Reads from the user program serial port over the system port
    pub async fn read_serial(
        &mut self,
        buf: &mut [u8],
    ) -> Result<usize, crate::errors::DecodeError> {
        // Optimization: Only read more bytes from the brain if we need them. This allows usages
        // that use small reads to be much faster.
        if self.read_buffer.len() < buf.len() {
            // Form a custom Extended command to read and write from serial.
            // We do the same as PROS, reading 64 bytes and specifying upload channel
            // Except we only read up to 64 bytes at a time, so that the user can configure if they want to
            // read smaller chunks (and thus bypass CRC errors from packet corruption, at the expense of speed)
            let payload = vec![
                crate::v5::V5ControllerChannel::Download as u8,
                u8::min(0x40, self.user_read_size),
            ];

            // Send the extended command 0x27
            let res = self
                .send_request(crate::commands::Extended(0x27, &payload))
                .await?;

            // Ensure that the response is for the correct command
            if res.0 != 0x27 {
                return Err(crate::errors::DecodeError::ExpectedCommand(0x27, res.0));
            }

            // The response payload should be the data that we read, so copy it into the read buffer
            // Discarding the first byte like pros does
            self.read_buffer.extend(&res.1[1..]);
        }

        // The amount of data to read into the buf
        let data_len = usize::min(buf.len(), self.read_buffer.len());

        // Get the data from the read buffer
        let mut data = self.read_buffer[..data_len].to_vec();

        // Pad it to the length of buf with 0s
        data.resize(buf.len(), 0);

        // Strip the data from the read buffer
        self.read_buffer = self.read_buffer[data_len..].to_vec();

        // Copy the first bytes of the read_buffer into buf, maxing out at the length of buf.
        // We do this so no data is lost
        buf.copy_from_slice(&data);

        // Return the length of the data we read
        Ok(data_len)
    }
}

impl<S: AsyncReadExt + AsyncWriteExt, U: AsyncReadExt + AsyncWriteExt> Unpin for AsyncDevice<S, U> {}

impl<S, U> AsyncRead for AsyncDevice<S, U>
where
    S: AsyncReadExt + AsyncWriteExt + AsyncRead,
    U: AsyncReadExt + AsyncWriteExt + AsyncRead + Unpin,
{
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

impl<S, U> AsyncWrite for AsyncDevice<S, U>
where
    S: AsyncReadExt + AsyncWriteExt,
    U: AsyncReadExt + AsyncWriteExt + Unpin,
{
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
