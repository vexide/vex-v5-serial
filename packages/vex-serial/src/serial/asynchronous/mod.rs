//! Implements discovering, opening, and interacting with vex devices connected over USB. This module does not have async support.

mod transport;

use futures_util::{AsyncReadExt, AsyncWriteExt, FutureExt, io::BufReader};
use transport::AsyncSerialPort;

use log::{error, trace, warn};
use std::{io::ErrorKind, time::Duration};
use vex_cdc::{Decode, Encode, FixedVec, VarU16, cdc::CdcReply, cdc2::system::UserDataPacket};

use crate::{
    AsyncConnection, ConnectionType,
    packet::{Frame, trim_packets},
    serial::{
        SerialDevice, SerialError, V5_SERIAL_BAUDRATE, VexSerialPort,
        asynchronous::transport::AsyncSerialPortBuilder,
    },
};

/// An open serial connection to a V5 device.
#[derive(Debug)]
pub struct AsyncSerialConnection {
    system_port: (VexSerialPort, AsyncSerialPort),
    user_port: Option<(VexSerialPort, BufReader<AsyncSerialPort>)>,
    incoming_packets: Vec<Frame>,
}

impl AsyncSerialConnection {
    /// Opens a new serial connection to a V5 Brain.
    pub fn open(device: SerialDevice) -> Result<Self, SerialError> {
        Ok(Self {
            system_port: {
                let stream =
                    serialport::new(&device.system_port.port_info.port_name, V5_SERIAL_BAUDRATE)
                        .parity(serialport::Parity::None)
                        .stop_bits(serialport::StopBits::One)
                        .timeout(Duration::from_secs(10))
                        .open_async()?;

                (device.system_port, stream)
            },
            user_port: if let Some(port) = device.user_port {
                let stream = serialport::new(&port.port_info.port_name, V5_SERIAL_BAUDRATE)
                    .parity(serialport::Parity::None)
                    .stop_bits(serialport::StopBits::One)
                    .timeout(Duration::from_secs(10))
                    .open_async()?;

                Some((port, BufReader::new(stream)))
            } else {
                None
            },
            incoming_packets: Default::default(),
        })
    }

    pub fn system_port(&self) -> &VexSerialPort {
        &self.system_port.0
    }

    pub fn user_port(&self) -> Option<&VexSerialPort> {
        self.user_port.as_ref().map(|port| &port.0)
    }

    /// Receives a single packet from the serial port and adds it to the queue of incoming packets.
    async fn receive_one_packet(&mut self) -> Result<(), SerialError> {
        // Read the header into an array
        let mut header = [0u8; 2];
        self.system_port.1.read_exact(&mut header).await?;

        // Verify that the header is valid
        if header != [0xAA, 0x55] {
            warn!("Skipping packet with invalid header: {:x?}.", header,);
            return Ok(());
        }

        // Create a buffer to store the entire packet
        let mut packet = Vec::from(header);

        let mut byte_buf = [0_u8; 1];

        // Push the command's ID
        packet.push({
            self.system_port.1.read_exact(&mut byte_buf).await?;
            byte_buf[0]
        });

        // Get the size of the packet
        // We do some extra logic to make sure we only read the necessary amount of bytes
        let first_size_byte = {
            self.system_port.1.read_exact(&mut byte_buf).await?;
            byte_buf[0]
        };
        let size = if VarU16::check_wide(first_size_byte) {
            let second_size_byte = {
                self.system_port.1.read_exact(&mut byte_buf).await?;
                byte_buf[0]
            };
            packet.extend([first_size_byte, second_size_byte]);

            // Decode the size of the packet
            VarU16::decode(&mut [first_size_byte, second_size_byte].as_slice())?
        } else {
            packet.push(first_size_byte);

            // Decode the size of the packet
            VarU16::decode(&mut [first_size_byte].as_slice())?
        }
        .into_inner() as usize;

        // Read the rest of the packet
        let mut payload = vec![0; size];
        self.system_port.1.read_exact(&mut payload).await?;

        // Completely fill the packet
        packet.extend(payload);

        trace!("received packet: {:x?}", packet);

        // Push the packet to the incoming packets buffer
        self.incoming_packets.push(Frame::new(packet));

        Ok(())
    }
}

impl AsyncConnection for AsyncSerialConnection {
    type Error = SerialError;

    fn connection_type(&self) -> ConnectionType {
        if self.user_port.is_some() {
            ConnectionType::Wired
        } else {
            ConnectionType::Controller
        }
    }

    async fn send(&mut self, packet: impl Encode) -> Result<(), SerialError> {
        // Encode the packet
        let mut encoded = vec![0; packet.size()];
        packet.encode(&mut encoded);

        trace!("sent packet: {:x?}", encoded);

        // Write the packet to the serial port
        self.system_port.1.write_all(&encoded).await?;
        self.system_port.1.flush().await?;

        Ok(())
    }

    async fn recv<P: CdcReply>(&mut self, timeout: Duration) -> Result<P, SerialError> {
        // Return an error if the right packet is not received within the timeout
        futures_util::select! {
            result = async {
                loop {
                    for packet in self.incoming_packets.iter_mut() {
                        match packet.decode_and_use::<P>() {
                            Some(Ok(decoded)) => {
                                trim_packets(&mut self.incoming_packets);
                                return Ok(decoded);
                            }
                            Some(Err(e)) => {
                                error!("Failed to decode packet with valid header: {}", e);
                                packet.used = true;
                                return Err(SerialError::DecodeError(e));
                            }
                            None => {}
                        }
                    }

                    trim_packets(&mut self.incoming_packets);
                    self.receive_one_packet().await?;
                }
            }.fuse() => result,
            _ = async_io::Timer::after(timeout).fuse() => Err(SerialError::IoError(std::io::Error::new(
                    ErrorKind::TimedOut,
                    "timed out waiting for reply from device",
                )))
        }
    }

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, SerialError> {
        if let Some((_, user_port)) = &mut self.user_port {
            Ok(user_port.read(buf).await?)
        } else {
            let mut data = Vec::new();
            loop {
                let fifo = self
                    .handshake(
                        UserDataPacket {
                            channel: 1, // stdio channel
                            write: FixedVec::new(),
                        },
                        Duration::from_millis(100),
                        0,
                    )
                    .await??;

                if !fifo.data.is_empty() {
                    data.extend(fifo.data);
                    break;
                }
            }

            let len = data.len().min(buf.len());
            buf[..len].copy_from_slice(&data[..len]);

            Ok(len)
        }
    }

    async fn write_user(&mut self, mut buf: &[u8]) -> Result<usize, SerialError> {
        if let Some((_, user_port)) = &mut self.user_port {
            let user_port = user_port.get_mut();
            user_port.write_all(buf).await?;
            user_port.flush().await?;
            Ok(buf.len())
        } else {
            let buf_len = buf.len();
            while !buf.is_empty() {
                let (chunk, rest) = buf.split_at(std::cmp::min(224, buf.len()));
                _ = self
                    .handshake(
                        UserDataPacket {
                            channel: 1, // stdio channel
                            write: FixedVec::from_slice(chunk).unwrap(),
                        },
                        Duration::from_millis(100),
                        0,
                    )
                    .await??;
                buf = rest;
            }

            Ok(buf_len)
        }
    }
}
