//! Implements discovering, opening, and interacting with vex devices connected over USB. This module does not have async support.

use log::{error, trace, warn};
use std::{
    io::{BufReader, ErrorKind, Read, Write},
    time::{Duration, Instant},
};
use vex_cdc::{Decode, Encode, FixedVec, VarU16, cdc::CdcReply, cdc2::system::UserDataPacket};

// these are re-exported so consumers can access USB port info, because SerialPortType must be named to unwrap UsbPortInfo
pub use serialport::SerialPort;

use crate::{
    Connection, ConnectionType,
    packet::{Frame, trim_packets},
    serial::{SerialDevice, SerialError, V5_SERIAL_BAUDRATE, VexSerialPort},
};

type PlatformSerialPort = cfg_select! {
    unix => serialport::TTYPort,
    windows => serialport::COMPort,
};

/// An open serial connection to a V5 device.
#[derive(Debug)]
pub struct SerialConnection {
    system_port: (VexSerialPort, PlatformSerialPort),
    user_port: Option<(VexSerialPort, BufReader<PlatformSerialPort>)>,
    incoming_packets: Vec<Frame>,
}

impl SerialConnection {
    /// Opens a new serial connection to a V5 Brain.
    pub fn open(device: SerialDevice) -> Result<Self, SerialError> {
        Ok(Self {
            system_port: {
                let stream =
                    serialport::new(&device.system_port.port_info.port_name, V5_SERIAL_BAUDRATE)
                        .parity(serialport::Parity::None)
                        .stop_bits(serialport::StopBits::One)
                        .open_native()?;

                (device.system_port, stream)
            },
            user_port: if let Some(port) = device.user_port {
                let stream = serialport::new(&port.port_info.port_name, V5_SERIAL_BAUDRATE)
                    .parity(serialport::Parity::None)
                    .stop_bits(serialport::StopBits::One)
                    .open_native()?;

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
    fn receive_one_packet(&mut self) -> Result<(), SerialError> {
        // Read the header into an array
        let mut header = [0u8; 2];
        self.system_port.1.read_exact(&mut header)?;

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
            self.system_port.1.read_exact(&mut byte_buf)?;
            byte_buf[0]
        });

        // Get the size of the packet
        // We do some extra logic to make sure we only read the necessary amount of bytes
        let first_size_byte = {
            self.system_port.1.read_exact(&mut byte_buf)?;
            byte_buf[0]
        };
        let size = if VarU16::check_wide(first_size_byte) {
            let second_size_byte = {
                self.system_port.1.read_exact(&mut byte_buf)?;
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
        self.system_port.1.read_exact(&mut payload)?;

        // Completely fill the packet
        packet.extend(payload);

        trace!("received packet: {:x?}", packet);

        // Push the packet to the incoming packets buffer
        self.incoming_packets.push(Frame::new(packet));

        Ok(())
    }
}

impl Connection for SerialConnection {
    type Error = SerialError;

    fn connection_type(&self) -> ConnectionType {
        if self.user_port.is_some() {
            ConnectionType::Wired
        } else {
            ConnectionType::Controller
        }
    }

    fn send(&mut self, packet: impl Encode) -> Result<(), SerialError> {
        // Encode the packet
        let mut encoded = vec![0; packet.size()];
        packet.encode(&mut encoded);

        trace!("sent packet: {:x?}", encoded);

        // Write the packet to the serial port
        match self.system_port.1.write_all(&encoded) {
            Ok(_) => (),
            Err(e) => return Err(SerialError::IoError(e)),
        };

        match self.system_port.1.flush() {
            Ok(_) => (),
            Err(e) => return Err(SerialError::IoError(e)),
        };

        Ok(())
    }

    fn recv<P: CdcReply>(&mut self, timeout: Duration) -> Result<P, SerialError> {
        let deadline = Instant::now() + timeout;

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

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SerialError::IoError(std::io::Error::new(
                    ErrorKind::TimedOut,
                    "timed out waiting for reply from device",
                )));
            }

            // Floor at 1ms so a tiny remainder isn't treated as a non-blocking read.
            self.system_port
                .1
                .set_timeout(remaining.max(Duration::from_millis(1)))?;
            self.receive_one_packet()?;
            self.system_port.1.set_timeout(Duration::ZERO)?;
        }
    }

    fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, SerialError> {
        if let Some((_, user_port)) = &mut self.user_port {
            Ok(match user_port.read(buf) {
                Ok(size) => size,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => 0,
                Err(e) => return Err(e.into()),
            })
        } else {
            let mut data = Vec::new();
            loop {
                let fifo = self.handshake(
                    UserDataPacket {
                        channel: 1, // stdio channel
                        write: FixedVec::new(),
                    },
                    Duration::from_millis(100),
                    0,
                )??;

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

    fn write_user(&mut self, mut buf: &[u8]) -> Result<usize, SerialError> {
        if let Some((_, user_port)) = &mut self.user_port {
            let user_port = user_port.get_mut();
            user_port.write_all(buf)?;
            user_port.flush()?;
            Ok(buf.len())
        } else {
            let buf_len = buf.len();
            while !buf.is_empty() {
                let (chunk, rest) = buf.split_at(std::cmp::min(224, buf.len()));
                _ = self.handshake(
                    UserDataPacket {
                        channel: 1, // stdio channel
                        write: FixedVec::from_slice(chunk).unwrap(),
                    },
                    Duration::from_millis(100),
                    0,
                )??;
                buf = rest;
            }

            Ok(buf_len)
        }
    }
}
