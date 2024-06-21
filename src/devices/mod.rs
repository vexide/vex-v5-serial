//! Implements functions and structures for interacting with vex devices.

use thiserror::Error;

use crate::{decode::DecodeError, encode::EncodeError, packets::cdc2::Cdc2Ack};

pub mod bluetoothv5;
pub mod device;
pub mod genericv5;

/// The default timeout for a serial connection in seconds
pub const SERIAL_TIMEOUT_SECONDS: u64 = 30;

/// The default timeout for a serial connection in nanoseconds
pub const SERIAL_TIMEOUT_NS: u32 = 0;

/// The USB PID of the V5 Brain
const VEX_V5_BRAIN_USB_PID: u16 = 0x0501;

/// The USB PID of the V5 Controller
const VEX_V5_CONTROLLER_USB_PID: u16 = 0x0503;

/// The USB VID for Vex devices
const VEX_USB_VID: u16 = 0x2888;

/// This enum represents three types of Vex serial devices:
/// The User port for communication with the user program.
/// The System port for communicating with VexOS.
/// And the Controller port for communicating with the VexV5 joystick
#[derive(PartialEq, Debug, Clone)]
pub enum VexPortType {
    User,
    System,
    Controller,
}

/// The type of a vex device
#[derive(Clone, Debug)]
pub enum VexDeviceType {
    Brain,
    Controller,
    Unknown,
}

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
    #[error("Serialport Error")]
    SerialportError(#[from] tokio_serial::Error),
    #[error("The user port can not be written to over wireless")]
    NoWriteOnWireless,
    #[error("Bluetooth Error")]
    BluetoothError(#[from] bluest::Error),
    #[error("The device is not a supported vex device")]
    InvalidDevice,
    #[error("Invalid Magic Number")]
    InvalidMagic,
    #[error("Not connected to the device")]
    NotConnected,
    #[error("No Bluetooth Adapter Found")]
    NoBluetoothAdapter,
}

/// This struct represents generic serial information for a vex device
#[derive(Clone, Debug)]
pub struct VexDevice {
    /// The platform-specific name of the system port
    pub system_port: String,

    /// The platform-specific name of the user port
    pub user_port: Option<String>,

    /// The type of the device
    pub device_type: VexDeviceType,
}

impl VexDevice {
    /// Open the device
    pub fn open(&self) -> Result<device::Device, DeviceError> {
        // Open the system port
        let system_port = match tokio_serial::SerialStream::open(
            &tokio_serial::new(&self.system_port, 115200)
                .parity(tokio_serial::Parity::None)
                .timeout(std::time::Duration::new(
                    crate::devices::SERIAL_TIMEOUT_SECONDS,
                    crate::devices::SERIAL_TIMEOUT_NS,
                ))
                .stop_bits(tokio_serial::StopBits::One),
        ) {
            Ok(v) => Ok(v),
            Err(e) => Err(DeviceError::SerialportError(e)),
        }?;

        // Open the user port (if it exists)

        let user_port = if let Some(port) = &self.user_port {
            Some(match tokio_serial::SerialStream::open(
                &tokio_serial::new(port, 115200)
                    .parity(tokio_serial::Parity::None)
                    .timeout(std::time::Duration::new(
                        crate::devices::SERIAL_TIMEOUT_SECONDS,
                        crate::devices::SERIAL_TIMEOUT_NS,
                    ))
                    .stop_bits(tokio_serial::StopBits::One),
            ) {
                Ok(v) => Ok(v),
                Err(e) => Err(DeviceError::SerialportError(e)),
            }?)
        } else {
            None
        };

        // Create the device
        let dev = device::Device::new(system_port, user_port);

        // Return the device
        Ok(dev)
    }
}
