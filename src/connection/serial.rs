//! Implements discovering, opening, and interacting with vex devices connected over USB. This module does not have async support.

use log::{debug, error, trace, warn};
use serialport::{SerialPortInfo, SerialPortType};
use std::time::Duration;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    select,
    time::sleep,
};
use tokio_serial::SerialStream;

use super::{CheckHeader, Connection, ConnectionType};
use crate::{
    connection::{trim_packets, RawPacket},
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    packets::{
        cdc2::Cdc2Ack,
        controller::{UserFifoPacket, UserFifoPayload, UserFifoReplyPacket}, HOST_BOUND_HEADER,
    },
    string::FixedString,
    varint::VarU16,
};

/// The USB venddor ID for VEX devices
pub const VEX_USB_VID: u16 = 0x2888;

/// The USB PID of the V5 Brain
pub const V5_BRAIN_USB_PID: u16 = 0x0501;

/// The USB PID of the EXP Brain
pub const EXP_BRAIN_USB_PID: u16 = 0x600;

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

/// Assigns port types by port location.
/// This does not appear to work on windows due to its shitty serial device drivers from 2006.
fn types_by_location(ports: &[SerialPortInfo]) -> Option<Vec<VexSerialPort>> {
    debug!("Attempting to infer serial port types by port location.");
    let mut vex_ports = Vec::new();

    for port in ports {
        // Get the info about the usb connection
        // This is always going to succeed because of earlier code.
        let SerialPortType::UsbPort(info) = port.clone().port_type else {
            continue;
        };

        if cfg!(target_os = "macos") && port.port_name.starts_with("/dev/tty.") {
            // https://pbxbook.com/other/mac-tty.html
            debug!(
                "Ignoring port named {:?} because it is a call-in device",
                port.port_name
            );
            continue;
        }

        match info.pid {
            V5_CONTROLLER_USB_PID => vex_ports.push(VexSerialPort {
                port_info: port.clone(),
                port_type: VexSerialPortType::Controller,
            }),
            V5_BRAIN_USB_PID | EXP_BRAIN_USB_PID => {
                // Check the product name for identifying information
                // This will not work on windows
                if let Some(mut location) = info.interface {
                    if cfg!(target_os = "macos") {
                        location -= 1; // macOS is 1-indexed
                    }

                    match location {
                        0 => {
                            debug!("Found a 'system' serial port over a Brain connection.");
                            vex_ports.push(VexSerialPort {
                            port_info: port.clone(),
                            port_type: VexSerialPortType::System,
                        })},
                        1 => warn!("Found a controller serial port over a Brain connection! Things are most likely broken."),
                        2 => {
                            debug!("Found a 'user' serial port over a Brain connection.");
                            vex_ports.push(VexSerialPort {
                            port_info: port.clone(),
                            port_type: VexSerialPortType::User,
                        })},
                        _ => warn!("Unknown location for V5 device: {}", location),
                    }
                }
            }
            // Unknown product
            _ => {}
        }
    }

    Some(vex_ports)
}

/// Assign port types based on the last character of the port name.
/// This is the fallback option for macOS.
/// This is a band-aid solution and will become obsolete once serialport correctly gets the interface number.
fn types_by_name_darwin(ports: &[SerialPortInfo]) -> Option<Vec<VexSerialPort>> {
    assert!(cfg!(target_os = "macos"));

    debug!("Attempting to infer serial port types by name. (Darwin fallback)");
    let mut vex_ports = Vec::new();

    for port in ports {
        if cfg!(target_os = "macos") && port.port_name.starts_with("/dev/tty.") {
            // https://pbxbook.com/other/mac-tty.html
            debug!(
                "Ignoring port named {:?} because it is a call-in device",
                port.port_name
            );
            continue;
        }

        let Some(interface) = port.port_name.chars().last() else {
            continue;
        };
        match interface {
            '1' => {
                debug!("Found a 'system' serial port over a Brain connection.");
                vex_ports.push(VexSerialPort {
                    port_info: port.clone(),
                    port_type: VexSerialPortType::System,
                });
            }
            '2' => {
                debug!("Found a controller serial port.");
                vex_ports.push(VexSerialPort {
                    port_info: port.clone(),
                    port_type: VexSerialPortType::Controller,
                });
            }
            '3' => {
                debug!("Found a 'user' serial port over a Brain connection.");
                vex_ports.push(VexSerialPort {
                    port_info: port.clone(),
                    port_type: VexSerialPortType::User,
                });
            }
            _ => {
                warn!("Unknown location for V5 device: {}", interface);
            }
        }
    }

    Some(vex_ports)
}

/// Infers port type by numerically sorting port product names.
/// This is the fallback option for windows.
/// The lower number port name is usually the user port according to pros-cli comments:
/// [https://github.com/purduesigbots/pros-cli/blob/develop/pros/serial/devices/vex/v5_device.py#L75]
fn types_by_name_order(ports: &[SerialPortInfo]) -> Option<Vec<VexSerialPort>> {
    debug!("Attempting to infer serial port types by order. (Windows fallback)");
    if ports.len() != 2 {
        return None;
    }

    let mut vex_ports = Vec::new();

    let mut sorted_ports = ports.to_vec();
    // Sort by product name
    sorted_ports.sort_by_key(|info| info.port_name.clone());
    sorted_ports.reverse();

    // Higher Port
    vex_ports.push(VexSerialPort {
        port_info: sorted_ports.pop().unwrap(),
        port_type: VexSerialPortType::System,
    });
    // Lower port
    vex_ports.push(VexSerialPort {
        port_info: sorted_ports.pop().unwrap(),
        port_type: VexSerialPortType::User,
    });

    // If we could not infer the type of all connections, fail
    if vex_ports.len() != ports.len() {
        return None;
    }

    Some(vex_ports)
}

/// Finds all available VEX serial ports that can be connected to.
fn find_ports() -> Result<Vec<VexSerialPort>, SerialError> {
    // Get all available serial ports
    let ports = tokio_serial::available_ports()?;

    // Create a vector that will contain all vex ports
    let mut filtered_ports = Vec::new();

    // Iterate over all available ports
    for port in ports {
        // Get the serial port's info as long as it is a usb port.
        // If it is not a USB port, ignore it.
        let SerialPortType::UsbPort(port_info) = port.clone().port_type else {
            continue;
        };

        // If the Vendor ID does not match the VEX Vendor ID, then skip it
        if port_info.vid != VEX_USB_VID {
            continue;
        }

        filtered_ports.push(port);
    }

    let vex_ports = types_by_location(&filtered_ports)
        .or_else(|| {
            if cfg!(target_os = "macos") {
                types_by_name_darwin(&filtered_ports)
            } else {
                types_by_name_order(&filtered_ports)
            }
        })
        .ok_or(SerialError::CouldntInferTypes)?;

    Ok(vex_ports)
}

/// Finds all connected V5 devices.
pub fn find_devices() -> Result<Vec<SerialDevice>, SerialError> {
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
    pub fn connect(&self, timeout: Duration) -> Result<SerialConnection, SerialError> {
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

/// Decodes a [`HostBoundPacket`]'s header sequence.
fn decode_header(data: impl IntoIterator<Item = u8>) -> Result<[u8; 2], DecodeError> {
    let mut data = data.into_iter();
    let header = Decode::decode(&mut data)?;
    if header != HOST_BOUND_HEADER {
        return Err(DecodeError::InvalidHeader);
    }
    Ok(header)
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
    pub fn open(device: SerialDevice, timeout: Duration) -> Result<Self, SerialError> {
        // Open the system port
        let system_port = match tokio_serial::SerialStream::open(
            &tokio_serial::new(device.system_port(), 115200)
                .parity(tokio_serial::Parity::None)
                .timeout(timeout)
                .stop_bits(tokio_serial::StopBits::One),
        ) {
            Ok(v) => Ok(v),
            Err(e) => Err(SerialError::SerialportError(e)),
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
                Err(e) => Err(SerialError::SerialportError(e)),
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
    async fn receive_one_packet(&mut self) -> Result<(), SerialError> {
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

        trace!("received packet: {:x?}", packet);

        // Push the packet to the incoming packets buffer
        self.incoming_packets.push(RawPacket::new(packet));

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

    async fn send_packet(&mut self, packet: impl Encode) -> Result<(), SerialError> {
        // Encode the packet
        let encoded = packet.encode()?;

        trace!("sent packet: {:x?}", encoded);

        // Write the packet to the serial port
        match self.system_port.write_all(&encoded).await {
            Ok(_) => (),
            Err(e) => return Err(SerialError::IoError(e)),
        };

        match self.system_port.flush().await {
            Ok(_) => (),
            Err(e) => return Err(SerialError::IoError(e)),
        };

        Ok(())
    }

    async fn receive_packet<P: Decode + CheckHeader>(&mut self, timeout: Duration) -> Result<P, SerialError> {
        // Return an error if the right packet is not received within the timeout
        select! {
            result = async {
                loop {
                    for packet in self.incoming_packets.iter_mut() {
                        if packet.check_header::<P>() {
                            match packet.decode_and_use::<P>() {
                                Ok(decoded) => {
                                    trim_packets(&mut self.incoming_packets);
                                    return Ok(decoded);
                                }
                                Err(e) => {
                                    error!("Failed to decode packet with valid header: {}", e);
                                    packet.used = true;
                                    return Err(SerialError::DecodeError(e));
                                }
                            }
                        }
                    }
                    trim_packets(&mut self.incoming_packets);
                    self.receive_one_packet().await?;
                }
            } => result,
            _ = sleep(timeout) => Err(SerialError::Timeout)
        }
    }

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, SerialError> {
        if let Some(user_port) = &mut self.user_port {
            Ok(user_port.read(buf).await?)
        } else {
            let mut data = Vec::new();
            loop {
                let fifo = self
                    .packet_handshake::<UserFifoReplyPacket>(
                        Duration::from_millis(100),
                        1,
                        UserFifoPacket::new(UserFifoPayload {
                            channel: 1, // stdio channel
                            write: None,
                        }),
                    )
                    .await?
                    .try_into_inner()?;
                if let Some(read) = fifo.data {
                    data.extend(read.as_bytes());
                    break;
                }
            }

            let len = data.len().min(buf.len());
            buf[..len].copy_from_slice(&data[..len]);

            Ok(len)
        }
    }

    async fn write_user(&mut self, mut buf: &[u8]) -> Result<usize, SerialError> {
        if let Some(user_port) = &mut self.user_port {
            Ok(user_port.write(buf).await?)
        } else {
            let buf_len = buf.len();
            while !buf.is_empty() {
                let (chunk, rest) = buf.split_at(std::cmp::min(224, buf.len()));
                _ = self
                    .packet_handshake::<UserFifoReplyPacket>(
                        Duration::from_millis(100),
                        1,
                        UserFifoPacket::new(UserFifoPayload {
                            channel: 2, // stdio channel
                            write: Some(
                                FixedString::new(String::from_utf8(chunk.to_vec()).unwrap())
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

#[derive(Error, Debug)]
pub enum SerialError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Packet encoding error: {0}")]
    EncodeError(#[from] EncodeError),
    #[error("Packet decoding error: {0}")]
    DecodeError(#[from] DecodeError),
    #[error("Packet timeout")]
    Timeout,
    #[error("NACK received: {0:?}")]
    Nack(#[from] Cdc2Ack),
    #[error("Serialport Error")]
    SerialportError(#[from] tokio_serial::Error),
    #[error("Could not infer serial port types")]
    CouldntInferTypes,
}
