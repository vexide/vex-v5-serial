//! Implements discovering, opening, and interacting with vex devices connected over USB. This module does not have async support.

use super::{
    VexDevice, VexDeviceType, VexPortType, VEX_USB_VID, VEX_V5_BRAIN_USB_PID,
    VEX_V5_CONTROLLER_USB_PID,
};

/// The information of a generic vex serial port
#[derive(Clone, Debug)]
pub struct VexGenericSerialPort {
    pub port_info: tokio_serial::SerialPortInfo,
    pub port_type: VexPortType,
}

/// Finds all generic vex v5 ports connected to the computer over usb.
fn find_generic_ports() -> Result<Vec<VexGenericSerialPort>, crate::errors::DeviceError> {
    // Get all available serial ports
    let ports = tokio_serial::available_ports()?;

    // Create a vector that will contain all vex ports
    let mut vex_ports: Vec<VexGenericSerialPort> = Vec::new();

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

        // If the product ID is any of the vex product IDs, then add them.
        if port_info.pid == VEX_V5_CONTROLLER_USB_PID {
            // If it i sa controlle,r then add it
            vex_ports.push(VexGenericSerialPort {
                port_info: port,
                port_type: VexPortType::Controller,
            });
        } else if port_info.pid == VEX_V5_BRAIN_USB_PID {
            // If it is the brain add it to the list. But we also need to determine if it is a system or a user port.
            vex_ports.push(VexGenericSerialPort {
                port_info: port,
                port_type: {
                    // Get the product name
                    let name = match port_info.product {
                        Some(s) => s,
                        _ => continue,
                    };

                    // If the name contains User, it is a User port
                    if name.contains("User") {
                        VexPortType::User
                    } else if name.contains("Communications") {
                        // If the name contains Communications, is is a System port.
                        VexPortType::System
                    } else if match vex_ports.last() {
                        Some(p) => p.port_type == VexPortType::System,
                        _ => false,
                    } {
                        // PROS source code also hints that User will always be listed after System
                        VexPortType::User
                    } else {
                        // If the previous one was user or the vector is empty,
                        // The PROS source code says that this one is most likely System.
                        VexPortType::System
                    }
                },
            })
        }

        // If none of this works out, then just ignore the port
    }

    Ok(vex_ports)
}

/// Finds all generic V5 devices from their ports
pub fn find_generic_devices() -> Result<Vec<VexDevice>, crate::errors::DeviceError> {
    // Find all vex ports
    let ports = find_generic_ports()?;

    // Create a vector of all vex devices
    let mut vex_devices = Vec::<VexDevice>::new();

    // Create a peekable iterator over all of the vex ports
    let mut port_iter = ports.iter().peekable();

    // Manually use a while loop to iterate, so that we can peek and pop ahead
    while let Some(current_port) = port_iter.next() {
        // Find out what type it is so we can assign devices
        if current_port.port_type == VexPortType::System {
            // Peek the next port. If it is a user port, add it to a brain device. If not, add it to an unknown device
            if match port_iter.peek() {
                Some(p) => p.port_type == VexPortType::User,
                _ => false,
            } {
                vex_devices.push(VexDevice {
                    system_port: current_port.port_info.port_name.clone(),
                    user_port: Some(port_iter.next().unwrap().port_info.port_name.clone()),
                    device_type: VexDeviceType::Brain,
                });
            } else {
                // If there is only a system device, add a unknown V5 device
                vex_devices.push(VexDevice {
                    system_port: current_port.port_info.port_name.clone(),
                    user_port: None,
                    device_type: VexDeviceType::Unknown,
                });
            }
        } else if current_port.port_type == VexPortType::Controller {
            // If it is a controller port, then add a controller device, because controllers have only a single port
            vex_devices.push(VexDevice {
                system_port: current_port.port_info.port_name.clone(),
                user_port: None,
                device_type: VexDeviceType::Controller,
            });
        } else if current_port.port_type == VexPortType::User {
            // If it is a user port, do the same thing we do with a system port. Except ignore it if there is no other port.
            if match port_iter.peek() {
                Some(p) => p.port_type == VexPortType::System,
                _ => false,
            } {
                vex_devices.push(VexDevice {
                    system_port: port_iter.next().unwrap().port_info.port_name.clone(),
                    user_port: Some(current_port.port_info.port_name.clone()),
                    device_type: VexDeviceType::Brain,
                });
            }
        }

        // If it is not any of these, ignore it
    }

    // Return the devices
    Ok(vex_devices)
}
