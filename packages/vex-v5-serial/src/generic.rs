use crate::{Connection, ConnectionType, bluetooth, serial};
use futures::{TryFutureExt, try_join};
use std::time::Duration;
use thiserror::Error;
use vex_cdc::{Decode, DecodeError, Encode, FixedStringSizeError, cdc2::Cdc2Ack};

use super::{CheckHeader, bluetooth::BluetoothError, serial::SerialError};

pub enum GenericConnection {
    Bluetooth(bluetooth::BluetoothConnection),
    Serial(serial::SerialConnection),
}
impl Connection for GenericConnection {
    type Error = GenericError;

    fn connection_type(&self) -> ConnectionType {
        match self {
            GenericConnection::Bluetooth(_) => ConnectionType::Bluetooth,
            GenericConnection::Serial(s) => s.connection_type(),
        }
    }

    async fn send(&mut self, packet: impl Encode) -> Result<(), GenericError> {
        match self {
            GenericConnection::Bluetooth(c) => c.send(packet).await?,
            GenericConnection::Serial(s) => s.send(packet).await?,
        };
        Ok(())
    }

    async fn recv<P: Decode + CheckHeader>(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<P, GenericError> {
        Ok(match self {
            GenericConnection::Bluetooth(c) => c.recv(timeout).await?,
            GenericConnection::Serial(s) => s.recv(timeout).await?,
        })
    }

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, GenericError> {
        Ok(match self {
            GenericConnection::Bluetooth(c) => c.read_user(buf).await?,
            GenericConnection::Serial(s) => s.read_user(buf).await?,
        })
    }

    async fn write_user(&mut self, buf: &[u8]) -> Result<usize, GenericError> {
        Ok(match self {
            GenericConnection::Bluetooth(c) => c.write_user(buf).await?,
            GenericConnection::Serial(s) => s.write_user(buf).await?,
        })
    }
}
impl GenericConnection {
    /// Returns whether the connection is over bluetooth.
    pub fn is_bluetooth(&self) -> bool {
        self.connection_type().is_bluetooth()
    }
    /// Returns whether the connection is over serial.
    pub fn is_wired(&self) -> bool {
        self.connection_type().is_wired()
    }
    /// Returns whether the connection is a controller.
    pub fn is_controller(&self) -> bool {
        self.connection_type().is_controller()
    }

    /// Checks if the connection is paired.
    /// If the connection is not over bluetooth, this function will return an error.
    pub async fn is_paired(&self) -> Result<bool, GenericError> {
        match self {
            GenericConnection::Bluetooth(c) => Ok(c.is_paired().await?),
            GenericConnection::Serial(_) => Err(GenericError::PairingNotSupported),
        }
    }

    /// Requests pairing with the device.
    /// # Errors
    /// If the connection is not over bluetooth, this function will return an error.
    /// This function will also error if there is a communication error while requesting pairing.
    pub async fn request_pairing(&mut self) -> Result<(), GenericError> {
        match self {
            GenericConnection::Bluetooth(c) => Ok(c.request_pairing().await?),
            GenericConnection::Serial(_) => Err(GenericError::PairingNotSupported),
        }
    }

    /// Attempts to authenticate the pairing request with the given pin.
    /// If the connection is not over bluetooth, this function will return an error.
    pub async fn authenticate_pairing(&mut self, pin: [u8; 4]) -> Result<(), GenericError> {
        match self {
            GenericConnection::Bluetooth(c) => Ok(c.authenticate_pairing(pin).await?),
            GenericConnection::Serial(_) => Err(GenericError::PairingNotSupported),
        }
    }
}

impl From<bluetooth::BluetoothConnection> for GenericConnection {
    fn from(c: bluetooth::BluetoothConnection) -> Self {
        GenericConnection::Bluetooth(c)
    }
}
impl From<serial::SerialConnection> for GenericConnection {
    fn from(c: serial::SerialConnection) -> Self {
        GenericConnection::Serial(c)
    }
}

#[derive(Debug, Clone)]
pub enum GenericDevice {
    Bluetooth(bluetooth::BluetoothDevice),
    Serial(serial::SerialDevice),
}
impl GenericDevice {
    pub async fn connect(&self, timeout: Duration) -> Result<GenericConnection, GenericError> {
        match self.clone() {
            GenericDevice::Bluetooth(d) => Ok(GenericConnection::Bluetooth(d.connect().await?)),
            GenericDevice::Serial(d) => Ok(GenericConnection::Serial(d.connect(timeout)?)),
        }
    }
}
impl From<serial::SerialDevice> for GenericDevice {
    fn from(d: serial::SerialDevice) -> Self {
        GenericDevice::Serial(d)
    }
}
impl From<bluetooth::BluetoothDevice> for GenericDevice {
    fn from(d: bluetooth::BluetoothDevice) -> Self {
        GenericDevice::Bluetooth(d)
    }
}

pub async fn find_devices() -> Result<Vec<GenericDevice>, GenericError> {
    let res = try_join! {
        bluetooth_devices().map_err(GenericError::BluetoothError),
        serial_devices().map_err(GenericError::SerialError),
    }
    .map(|(bluetooth, serial)| bluetooth.into_iter().chain(serial.into_iter()).collect())?;
    Ok(res)
}

async fn bluetooth_devices() -> Result<Vec<GenericDevice>, BluetoothError> {
    // Scan for 10 seconds
    let devices = bluetooth::find_devices(Duration::from_secs(10), None).await?;
    let devices = devices.into_iter().map(GenericDevice::Bluetooth).collect();
    Ok(devices)
}

async fn serial_devices() -> Result<Vec<GenericDevice>, SerialError> {
    let devices = serial::find_devices()?;
    let devices = devices.into_iter().map(GenericDevice::Serial).collect();
    Ok(devices)
}

#[derive(Error, Debug)]
pub enum GenericError {
    #[error("Serial Error: {0}")]
    SerialError(#[from] SerialError),
    #[error("Bluetooth Error: {0}")]
    BluetoothError(#[from] BluetoothError),
    #[error("Packet decoding error: {0}")]
    DecodeError(#[from] DecodeError),
    #[error("NACK received: {0:?}")]
    Nack(#[from] Cdc2Ack),
    #[error("Pairing is not supported over any connection other than Bluetooth")]
    PairingNotSupported,
    #[error(transparent)]
    FixedStringSizeError(#[from] FixedStringSizeError),
}
