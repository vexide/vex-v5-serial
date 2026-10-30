#[cfg(feature = "serial-blocking")]
mod blocking;
#[cfg(feature = "serial-blocking")]
pub use blocking::SerialConnection;

#[cfg(feature = "serial-async")]
mod asynchronous;
#[cfg(feature = "serial-async")]
pub use asynchronous::AsyncSerialConnection;

// these are re-exported so consumers can access USB port info, because SerialPortType must be named to unwrap UsbPortInfo
pub use serialport::SerialPort;

use serialport::{SerialPortInfo, SerialPortType};
use thiserror::Error;
use vex_cdc::{DecodeError, FixedStringSizeError, cdc2::Cdc2Ack};

use crate::ConnectionError;

/// The USB vendor ID for VEX devices
pub const VEX_USB_VID: u16 = 0x2888;

/// The USB PID of the V5 Brain
pub const V5_BRAIN_USB_PID: u16 = 0x0501;

/// The USB PID of the EXP Brain
pub const EXP_BRAIN_USB_PID: u16 = 0x600;

/// The USB PID of the V5 Controller
pub const V5_CONTROLLER_USB_PID: u16 = 0x0503;

pub const AIR_HORNET_USB_PID: u16 = 0x0a00;

pub const AIR_CONTROLLER_USB_PID: u16 = 0x0a10;

pub const AIM_USB_PID: u16 = 0x0700;

/// The AIM's user port; interfaces 2 and 3 are its camera.
const AIM_USER_INTERFACE: u8 = 4;

pub const AIV_USB_PID: u16 = 0x0800;

pub const V5_SERIAL_BAUDRATE: u32 = 115200;

/// The information of a generic vex serial port
#[derive(Clone, Debug)]
pub struct VexSerialPort {
    pub port_info: serialport::SerialPortInfo,
    pub port_type: VexSerialPortType,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VexSerialPortType {
    User,
    System,
    Controller,
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
                // Peek the next port. If it is a user port, add it to a brain device. If not, add it to an unknown device
                if match ports.peek() {
                    Some(p) => p.port_type == VexSerialPortType::User,
                    _ => false,
                } {
                    devices.push(SerialDevice {
                        system_port: port,
                        user_port: ports.next(),
                    });
                } else {
                    devices.push(SerialDevice {
                        system_port: port,
                        user_port: None,
                    });
                }
            }
            VexSerialPortType::User => {
                // If it is a user port, do the same thing we do with a system port. Except ignore it if there is no other port.
                if match ports.peek() {
                    Some(p) => p.port_type == VexSerialPortType::System,
                    _ => false,
                } {
                    devices.push(SerialDevice {
                        system_port: ports.next().unwrap(),
                        user_port: Some(port),
                    });
                }
            }
            VexSerialPortType::Controller => devices.push(SerialDevice {
                system_port: port,
                user_port: None,
            }),
        }
    }

    // Return the devices
    Ok(devices)
}

//
// MARK: Port enumeration
//

/// Finds all available VEX serial ports that can be connected to.
fn find_ports() -> Result<Vec<VexSerialPort>, SerialError> {
    // Get all available serial ports
    let ports = serialport::available_ports()?;

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
        .unwrap_or(Vec::new());

    Ok(vex_ports)
}

/// Assigns port types by port location.
/// This does not appear to work on windows due to its shitty serial device drivers from 2006.
fn types_by_location(ports: &[SerialPortInfo]) -> Option<Vec<VexSerialPort>> {
    log::debug!("Attempting to infer serial port types by port location.");
    let mut vex_ports = Vec::new();

    for port in ports {
        // Get the info about the usb connection
        // This is always going to succeed because of earlier code.
        let SerialPortType::UsbPort(info) = port.clone().port_type else {
            continue;
        };

        if cfg!(target_os = "macos") && port.port_name.starts_with("/dev/tty.") {
            // https://pbxbook.com/other/mac-tty.html
            log::debug!(
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
            V5_BRAIN_USB_PID
            | EXP_BRAIN_USB_PID
            | AIR_CONTROLLER_USB_PID
            | AIR_HORNET_USB_PID
            | AIM_USB_PID
            | AIV_USB_PID => {
                // Check the product name for identifying information
                // This will not work on windows

                if let Some(mut location) = info.interface {
                    if cfg!(target_os = "macos") {
                        location -= 1; // macOS is 1-indexed
                    }

                    if info.pid == AIM_USB_PID && location == AIM_USER_INTERFACE {
                        log::debug!("Found a 'user' serial port over an AIM connection.");
                        vex_ports.push(VexSerialPort {
                            port_info: port.clone(),
                            port_type: VexSerialPortType::User,
                        });
                        continue;
                    }

                    match location {
                        0 => {
                            log::debug!("Found a 'system' serial port over a Brain connection.");
                            vex_ports.push(VexSerialPort {
                                port_info: port.clone(),
                                port_type: VexSerialPortType::System,
                            })
                        }
                        1 => log::warn!(
                            "Found a controller serial port over a Brain connection! Things are most likely broken."
                        ),
                        2 => {
                            log::debug!("Found a 'user' serial port over a Brain connection.");
                            vex_ports.push(VexSerialPort {
                                port_info: port.clone(),
                                port_type: VexSerialPortType::User,
                            })
                        }
                        _ => log::warn!("Unknown location for V5 device: {}", location),
                    }
                }
            }
            // Unknown product
            _ => {}
        }
    }

    if vex_ports.is_empty() {
        return None;
    }

    Some(vex_ports)
}

/// Assign port types based on the last character of the port name.
/// This is the fallback option for macOS.
/// This is a band-aid solution and will become obsolete once serialport correctly gets the interface number.
fn types_by_name_darwin(ports: &[SerialPortInfo]) -> Option<Vec<VexSerialPort>> {
    assert!(cfg!(target_os = "macos"));

    log::debug!("Attempting to infer serial port types by name. (Darwin fallback)");
    let mut vex_ports = Vec::new();

    for port in ports {
        if cfg!(target_os = "macos") && port.port_name.starts_with("/dev/tty.") {
            // https://pbxbook.com/other/mac-tty.html
            log::debug!(
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
                log::debug!("Found a 'system' serial port over a Brain connection.");
                vex_ports.push(VexSerialPort {
                    port_info: port.clone(),
                    port_type: VexSerialPortType::System,
                });
            }
            '2' => {
                log::debug!("Found a controller serial port.");
                vex_ports.push(VexSerialPort {
                    port_info: port.clone(),
                    port_type: VexSerialPortType::Controller,
                });
            }
            '3' => {
                log::debug!("Found a 'user' serial port over a Brain connection.");
                vex_ports.push(VexSerialPort {
                    port_info: port.clone(),
                    port_type: VexSerialPortType::User,
                });
            }
            _ => {
                log::warn!("Unknown location for V5 device: {}", interface);
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
    log::debug!("Attempting to infer serial port types by order. (Windows fallback)");
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

#[derive(Clone, Debug)]
pub struct SerialDevice {
    system_port: VexSerialPort,
    user_port: Option<VexSerialPort>,
}

impl SerialDevice {
    #[cfg(feature = "serial-blocking")]
    pub fn connect(&self) -> Result<SerialConnection, SerialError> {
        SerialConnection::open(self.clone())
    }

    #[cfg(feature = "serial-async")]
    pub fn connect_async(&self) -> Result<AsyncSerialConnection, SerialError> {
        AsyncSerialConnection::open(self.clone())
    }

    pub fn system_port(&self) -> &VexSerialPort {
        &self.system_port
    }

    pub fn user_port(&self) -> Option<&VexSerialPort> {
        self.user_port.as_ref()
    }
}

#[derive(Error, Debug)]
pub enum SerialError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Packet decoding error: {0}")]
    DecodeError(#[from] DecodeError),

    #[error("NACK received: {0:?}")]
    Nack(#[from] Cdc2Ack),

    #[error("Serialport Error")]
    SerialportError(#[from] serialport::Error),

    #[error("Could not infer serial port types")]
    CouldntInferTypes,

    #[error(transparent)]
    FixedStringSizeError(#[from] FixedStringSizeError),
}

impl ConnectionError for SerialError {
    fn should_retry(&self) -> bool {
        matches!(self, Self::IoError(e) if e.kind() == std::io::ErrorKind::TimedOut)
    }
}
