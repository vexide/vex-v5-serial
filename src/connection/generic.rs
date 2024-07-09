use crate::{
    connection::{
        bluetooth,
        serial,
        Connection, ConnectionError, ConnectionType,
    },
    decode::Decode,
    encode::Encode,
};
use futures::try_join;
use std::time::Duration;

pub enum GenericConnection {
    Bluetooth(bluetooth::BluetoothConnection),
    Serial(serial::SerialConnection),
}
impl Connection for GenericConnection {
    fn connection_type(&self) -> ConnectionType {
        match self {
            GenericConnection::Bluetooth(_) => ConnectionType::Bluetooth,
            GenericConnection::Serial(s) => s.connection_type(),
        }
    }

    async fn send_packet(&mut self, packet: impl Encode) -> Result<(), ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.send_packet(packet).await,
            GenericConnection::Serial(s) => s.send_packet(packet).await,
        }
    }

    async fn receive_packet<P: Decode>(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<P, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.receive_packet(timeout).await,
            GenericConnection::Serial(s) => s.receive_packet(timeout).await,
        }
    }

    async fn read_user(&mut self, buf: &mut Vec<u8>) -> Result<usize, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.read_user(buf).await,
            GenericConnection::Serial(s) => s.read_user(buf).await,
        }
    }

    async fn write_user(&mut self, buf: &[u8]) -> Result<usize, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.write_user(buf).await,
            GenericConnection::Serial(s) => s.write_user(buf).await,
        }
    }
}
impl GenericConnection {
    /// Checks if the connection is paired.
    /// If the connection is not over bluetooth, this function will return an error.
    pub async fn is_paired(&self) -> Result<bool, ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.is_paired().await,
            GenericConnection::Serial(_) => Err(ConnectionError::PairingNotSupported),
        }
    }

    /// Requests pairing with the device.
    /// # Errors
    /// If the connection is not over bluetooth, this function will return an error.
    /// This function will also error if there is a communication error while requesting pairing.
    pub async fn request_pairing(&mut self) -> Result<(), ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.request_pairing().await,
            GenericConnection::Serial(_) => Err(ConnectionError::PairingNotSupported),
        }
    }

    /// Attempts to authenticate the pairing request with the given pin.
    /// If the connection is not over bluetooth, this function will return an error.
    pub async fn authenticate_pairing(&mut self, pin: [u8; 4]) -> Result<(), ConnectionError> {
        match self {
            GenericConnection::Bluetooth(c) => c.authenticate_pairing(pin).await,
            GenericConnection::Serial(_) => Err(ConnectionError::PairingNotSupported),
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
    pub async fn connect(&self, timeout: Duration) -> Result<GenericConnection, ConnectionError> {
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

pub async fn find_devices() -> Result<Vec<GenericDevice>, ConnectionError> {
    let res = try_join! {
        bluetooth_devices(),
        serial_devices(),
    }
    .map(|(bluetooth, serial)| bluetooth.into_iter().chain(serial.into_iter()).collect())?;
    Ok(res)
}

async fn bluetooth_devices() -> Result<Vec<GenericDevice>, ConnectionError> {
    // Scan for 10 seconds
    let devices = bluetooth::find_devices(Duration::from_secs(10), None).await?;
    let devices = devices.into_iter().map(GenericDevice::Bluetooth).collect();
    Ok(devices)
}

async fn serial_devices() -> Result<Vec<GenericDevice>, ConnectionError> {
    let devices = serial::find_devices()?;
    let devices = devices.into_iter().map(GenericDevice::Serial).collect();
    Ok(devices)
}
