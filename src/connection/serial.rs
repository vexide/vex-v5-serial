//! Implements discovering, opening, and interacting with vex devices connected over USB. This module does not have async support.

use log::{debug, trace, warn};
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    select,
    time::sleep,
};
use tokio_serial::SerialStream;

use super::{Connection, ConnectionError, ConnectionType};
use crate::{
    connection::{trim_packets, RawPacket},
    decode::Decode,
    encode::Encode,
    packets::{
        controller::{UserFifoPacket, UserFifoPayload, UserFifoReplyPacket},
        decode_header,
    },
    string::VarLengthString,
    varint::VarU16,
};

/// The USB venddor ID for VEX devices
pub const VEX_USB_VID: u16 = 0x2888;

/// The USB PID of the V5 Brain
pub const V5_BRAIN_USB_PID: u16 = 0x0501;

/// The USB PID of the V5 Controller
pub const V5_CONTROLLER_USB_PID: u16 = 0x0503;

pub const V5_SERIAL_BAUDRATE: u32 = 115200;

/// The information of a generic vex serial port
#[derive(Clone, Debug)]
pub struct VexSerialPort {
    pub port_info: tokio_serial::SerialPortInfo,
    pub port_type: VexSerialPortType,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VexSerialPortType {
    User,
    System,
    Controller,
}

/// Finds all available VEX serial ports that can be connected to.
fn find_ports() -> Result<Vec<VexSerialPort>, ConnectionError> {
    // Get all available serial ports
    let ports = tokio_serial::available_ports()?;

    // Create a vector that will contain all vex ports
    let mut vex_ports: Vec<VexSerialPort> = Vec::new();

    // Iterate over all available ports
    for port in ports {
        // Get the serial port's info as long as it is a usb port.
        // If it is not a USB port, ignore it.
        let port_info = match port.clone().port_type {
            tokio_serial::SerialPortType::UsbPort(info) => info,
            _ => continue, // Skip the port if it is not USB.
        };

        // If the Vendor ID does not match the VEX Vendor ID, then skip it
        if port_info.vid != VEX_USB_VID {
            continue;
        }

        match port_info.pid {
            V5_CONTROLLER_USB_PID => {
                // V5 controller
                vex_ports.push(VexSerialPort {
                    port_info: port,
                    port_type: VexSerialPortType::Controller,
                });
            }
            V5_BRAIN_USB_PID => {
                // V5 Brain System or User Port
                vex_ports.push(VexSerialPort {
                    port_info: port,
                    port_type: {
                        // Get the product name
                        let name = match port_info.product {
                            Some(s) => s,
                            _ => continue,
                        };

                        // If the name contains User, it is a User port
                        if name.contains("User") {
                            VexSerialPortType::User
                        } else if name.contains("Communications") {
                            // If the name contains Communications, is is a System port.
                            VexSerialPortType::System
                        } else if match vex_ports.last() {
                            Some(p) => p.port_type == VexSerialPortType::System,
                            _ => false,
                        } {
                            // PROS source code also hints that User will always be listed after System
                            VexSerialPortType::User
                        } else {
                            // If the previous one was user or the vector is empty,
                            // The PROS source code says that this one is most likely System.
                            VexSerialPortType::System
                        }
                    },
                });
            }
            _ => {}
        }
    }

    Ok(vex_ports)
}

/// Finds all connected V5 devices.
pub fn find_devices() -> Result<Vec<SerialDevice>, ConnectionError> {
    // Find all vex ports, iterate using peekable.
    let mut ports = find_ports()?.into_iter().peekable();

    // Create a vector of all vex devices
    let mut devices = Vec::<SerialDevice>::new();

    // Manually use a while loop to iterate, so that we can peek and pop ahead
    while let Some(port) = ports.next() {
        // Find out what type it is so we can assign devices
        match port.port_type {
            VexSerialPortType::System => {
                let port_name = port.port_info.port_name.clone();

                // Peek the next port. If it is a user port, add it to a brain device. If not, add it to an unknown device
                if match ports.peek() {
                    Some(p) => p.port_type == VexSerialPortType::User,
                    _ => false,
                } {
                    devices.push(SerialDevice::Brain {
                        system_port: port_name,
                        user_port: ports.next().unwrap().port_info.port_name.clone(),
                    });
                } else {
                    // If there is only a system device, add a unknown V5 device
                    devices.push(SerialDevice::Unknown {
                        system_port: port_name,
                    });
                }
            }
            VexSerialPortType::User => {
                // If it is a user port, do the same thing we do with a system port. Except ignore it if there is no other port.
                if match ports.peek() {
                    Some(p) => p.port_type == VexSerialPortType::System,
                    _ => false,
                } {
                    devices.push(SerialDevice::Brain {
                        system_port: ports.next().unwrap().port_info.port_name.clone(),
                        user_port: port.port_info.port_name.clone(),
                    });
                }
            }
            VexSerialPortType::Controller => devices.push(SerialDevice::Controller {
                system_port: port.port_info.port_name.clone(),
            }),
        }
    }

    // Return the devices
    Ok(devices)
}

/// Represents a V5 device that can be connected to over serial.
#[derive(Clone, Debug)]
pub enum SerialDevice {
    /// V5 Brain
    ///
    /// Has both a system and user port.
    Brain {
        user_port: String,
        system_port: String,
    },

    /// V5 Controller
    ///
    /// Has a system port, but no user port.
    Controller { system_port: String },

    /// Unknown V5 Peripheral.
    ///
    /// A secret, more sinsiter, third thing.
    /// *Probably doesn't even exist. How'd you even get this to happen?*
    ///
    /// Has a system port and no user port but __is not a controller__.
    Unknown { system_port: String },
}

impl SerialDevice {
    pub fn connect(&self, timeout: Duration) -> Result<SerialConnection, ConnectionError> {
        SerialConnection::open(self.clone(), timeout)
    }

    pub fn system_port(&self) -> String {
        match &self {
            Self::Brain {
                system_port,
                user_port: _,
            }
            | Self::Controller { system_port }
            | Self::Unknown { system_port } => system_port.clone(),
        }
    }

    pub fn user_port(&self) -> Option<String> {
        match &self {
            Self::Brain {
                system_port: _,
                user_port,
            } => Some(user_port.clone()),
            _ => None,
        }
    }
}

/// An open serial connection to a V5 device.
#[derive(Debug)]
pub struct SerialConnection {
    system_port: SerialStream,
    user_port: Option<BufReader<SerialStream>>,
    incoming_packets: Vec<RawPacket>,
}

impl SerialConnection {
    /// Opens a new serial connection to a V5 Brain.
    pub fn open(device: SerialDevice, timeout: Duration) -> Result<Self, ConnectionError> {
        // Open the system port
        let system_port = match tokio_serial::SerialStream::open(
            &tokio_serial::new(device.system_port(), 115200)
                .parity(tokio_serial::Parity::None)
                .timeout(timeout)
                .stop_bits(tokio_serial::StopBits::One),
        ) {
            Ok(v) => Ok(v),
            Err(e) => Err(ConnectionError::SerialportError(e)),
        }?;

        // Open the user port (if it exists)
        let user_port = if let Some(port) = &device.user_port() {
            Some(match tokio_serial::SerialStream::open(
                &tokio_serial::new(port, V5_SERIAL_BAUDRATE)
                    .parity(tokio_serial::Parity::None)
                    .timeout(timeout)
                    .stop_bits(tokio_serial::StopBits::One),
            ) {
                Ok(v) => Ok(BufReader::new(v)),
                Err(e) => Err(ConnectionError::SerialportError(e)),
            }?)
        } else {
            None
        };

        Ok(Self {
            system_port,
            user_port,
            incoming_packets: Default::default(),
        })
    }

    /// Receives a single packet from the serial port and adds it to the queue of incoming packets.
    async fn receive_one_packet(&mut self) -> Result<(), ConnectionError> {
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

        debug!("received packet: {:x?}", packet);

        // Push the packet to the incoming packets buffer
        self.incoming_packets.push(RawPacket::new(packet));

        Ok(())
    }
}

impl Connection for SerialConnection {
    fn connection_type(&self) -> ConnectionType {
        if self.user_port.is_some() {
            ConnectionType::Wired
        } else {
            ConnectionType::Controller
        }
    }

    async fn send_packet(&mut self, packet: impl Encode) -> Result<(), ConnectionError> {
        // Encode the packet
        let encoded = packet.encode()?;

        trace!("Sending packet: {:x?}", encoded);

        // Write the packet to the serial port
        match self.system_port.write_all(&encoded).await {
            Ok(_) => (),
            Err(e) => return Err(ConnectionError::IoError(e)),
        };

        match self.system_port.flush().await {
            Ok(_) => (),
            Err(e) => return Err(ConnectionError::IoError(e)),
        };

        Ok(())
    }

    async fn receive_packet<P: Decode>(&mut self, timeout: Duration) -> Result<P, ConnectionError> {
        // Return an error if the right packet is not received within the timeout
        select! {
            result = async {
                loop {
                    for packet in self.incoming_packets.iter_mut() {
                        if let Ok(decoded) = packet.decode_and_use::<P>() {
                            trim_packets(&mut self.incoming_packets);
                            return Ok(decoded);
                        }
                    }
                    trim_packets(&mut self.incoming_packets);
                    self.receive_one_packet().await?;
                }
            } => result,
            _ = sleep(timeout) => Err(ConnectionError::Timeout)
        }
    }

    async fn read_user(&mut self, buf: &mut Vec<u8>) -> Result<usize, ConnectionError> {
        if let Some(user_port) = &mut self.user_port {
            Ok(user_port.read_until(0xA, buf).await?)
        } else {
            let fifo = self
                .packet_handshake::<UserFifoReplyPacket>(
                    Duration::from_millis(100),
                    0,
                    UserFifoPacket::new(UserFifoPayload {
                        channel: 1, // stdio channel
                        read_length: 0x40,
                        write: None,
                    }),
                )
                .await?
                .try_into_inner()?;

            let mut data = std::io::Cursor::new(fifo.data.0.as_bytes());

            Ok(std::io::Read::read(&mut data, buf)?)
        }
    }

    async fn write_user(&mut self, mut buf: &[u8]) -> Result<usize, ConnectionError> {
        if let Some(user_port) = &mut self.user_port {
            Ok(user_port.write(buf).await?)
        } else {
            let buf_len = buf.len();
            while !buf.is_empty() {
                let (chunk, rest) = buf.split_at(std::cmp::min(224, buf.len()));
                _ = self
                    .packet_handshake::<UserFifoReplyPacket>(
                        Duration::from_millis(100),
                        0,
                        UserFifoPacket::new(UserFifoPayload {
                            channel: 1, // stdio channel
                            read_length: 0,
                            write: Some(
                                VarLengthString::new(String::from_utf8(chunk.to_vec()).unwrap())
                                    .unwrap(),
                            ),
                        }),
                    )
                    .await?
                    .try_into_inner()?;
                buf = rest;
            }

            Ok(buf_len)
        }

    }
}
