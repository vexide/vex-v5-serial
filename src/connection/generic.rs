use crate::{
    connection::{
        bluetooth::{self, BluetoothConnection},
        serial::{self, SerialConnection},
        Connection, ConnectionError, ConnectionType,
    },
    decode::Decode,
    encode::Encode,
};
use futures::try_join;
use std::time::Duration;

pub enum GenericConnection {
    Bluetooth(BluetoothConnection),
    Serial(SerialConnection),
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

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, ConnectionError> {
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

pub enum GenericDevice {
    Bluetooth(bluetooth::BluetoothDevice),
    Serial(serial::SerialDevice),
}
impl GenericDevice {
    pub async fn connect(self, timeout: Duration) -> Result<GenericConnection, ConnectionError> {
        match self {
            GenericDevice::Bluetooth(d) => Ok(GenericConnection::Bluetooth(d.connect().await?)),
            GenericDevice::Serial(d) => Ok(GenericConnection::Serial(d.connect(timeout)?)),
        }
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
