use std::time::{Duration, Instant};

use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use log::{debug, error, info, trace, warn};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::decode::Decode;
use crate::encode::Encode;

use super::{Connection, ConnectionError};

/// The BLE GATT Service that V5 Brains provide
pub const V5_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

/// User port GATT characteristic
pub const CHARACTERISTIC_TX_SYSTEM: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
pub const CHARACTERISTIC_RX_SYSTEM: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13f5); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

/// System port GATT characteristic
pub const CHARACTERISTIC_TX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
pub const CHARACTERISTIC_RX_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1326); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIF

/// PIN authentication characteristic
pub const CHARACTERISTIC_AUTH: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5); // READ | WRITE_WITHOUT_RESPONSE | WRITE

pub const AUTH_REQUIRED_SEQUENCE: u32 = 0xdeadface;

/// Discover and locate bluetooth-compatible V5 peripherals.
pub async fn find_devices(
    scan_time: Duration,
    max_device_count: Option<usize>,
) -> Result<Vec<Peripheral>, ConnectionError> {
    // Create a new bluetooth device manager.
    let manager = Manager::new().await?;

    // Use the first adapter we can find.
    let adapter = if let Some(adapter) = manager.adapters().await?.into_iter().nth(0) {
        adapter
    } else {
        // No bluetooth adapters were found.
        return Err(ConnectionError::NoBluetoothAdapter);
    };

    // Our bluetooth adapter will give us an event stream that can tell us when
    // a device is discovered. We can use this to get information on when a scan
    // has found a device.
    let mut events = adapter.events().await?;

    // List of devices that we'll add to during discovery.
    let mut devices = Vec::<Peripheral>::new();

    // Scan for peripherals using the V5 service UUID.
    let scan_start_time = Instant::now();
    adapter
        .start_scan(ScanFilter {
            services: vec![V5_SERVICE],
        })
        .await?;

    // Listen for events. When the adapter indicates that a device has been discovered,
    // we'll ensure that the peripheral is correct and add it to our device list.
    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                let peripheral = adapter.peripheral(&id).await?;

                if let Some(properties) = peripheral.properties().await? {
                    if properties.services.contains(&V5_SERVICE) {
                        // Assuming the peripheral contains the V5 service UUID, we have a brain.
                        debug!("Found V5 brain at {}", peripheral.address());

                        devices.push(peripheral);

                        // Break the discovery loop if we have found enough devices.
                        if let Some(count) = max_device_count {
                            if devices.len() == count {
                                break;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Also break if we've exceeded the provided scan time.
        if scan_start_time.elapsed() > scan_time {
            break;
        }
    }

    info!(
        "Found {} devices in {:?}",
        devices.len(),
        scan_start_time.elapsed()
    );

    Ok(devices)
}

pub struct BluetoothConnection {
    peripheral: Peripheral,
    tx_system: Characteristic,
    rx_system: Characteristic,
    tx_user: Characteristic,
    rx_user: Characteristic,
    auth: Characteristic,
}

impl BluetoothConnection {
    pub async fn open(peripheral: Peripheral) -> Result<Self, ConnectionError> {
        if !peripheral.is_connected().await? {
            peripheral.connect().await?;
        } else {
            warn!("Peripheral already connected?");
        }
    
        peripheral.discover_services().await?;

        let mut tx_system: Option<Characteristic> = None;
        let mut rx_system: Option<Characteristic> = None;
        let mut tx_user: Option<Characteristic> = None;
        let mut rx_user: Option<Characteristic> = None;
        let mut auth: Option<Characteristic> = None;

        for characteric in peripheral.characteristics() {
            match characteric.uuid {
                CHARACTERISTIC_TX_SYSTEM => {
                    tx_system = Some(characteric);
                }
                CHARACTERISTIC_RX_SYSTEM => {
                    rx_system = Some(characteric);
                }
                CHARACTERISTIC_TX_USER => {
                    tx_user = Some(characteric);
                }
                CHARACTERISTIC_RX_USER => {
                    rx_user = Some(characteric);
                }
                CHARACTERISTIC_AUTH => {
                    auth = Some(characteric);
                }
                _ => {}
            }
        }

        let connection = Self {
            peripheral,
            tx_system: tx_system.ok_or(ConnectionError::MissingCharacteristic)?,
            rx_system: rx_system.ok_or(ConnectionError::MissingCharacteristic)?,
            tx_user: tx_user.ok_or(ConnectionError::MissingCharacteristic)?,
            rx_user: rx_user.ok_or(ConnectionError::MissingCharacteristic)?,
            auth: auth.ok_or(ConnectionError::MissingCharacteristic)?,
        };

        connection.peripheral.subscribe(&connection.rx_system).await.ok();
        connection.peripheral.subscribe(&connection.rx_user).await.ok();

        Ok(connection)
    }

    pub async fn is_authenticated(&self) -> Result<bool, ConnectionError> {
        let auth_bytes = self.peripheral.read(&self.auth).await?;

        Ok(u32::from_be_bytes(auth_bytes[0..4].try_into().unwrap()) != AUTH_REQUIRED_SEQUENCE)
    }

    pub async fn request_pin(&mut self) -> Result<(), ConnectionError> {
        self.peripheral
            .write(
                &self.auth,
                &[0xFF, 0xFF, 0xFF, 0xFF],
                WriteType::WithoutResponse,
            )
            .await?;

        Ok(())
    }

    pub async fn authenticate(&mut self, pin: [u8; 4]) -> Result<(), ConnectionError> {
        self.peripheral
            .write(&self.auth, &pin, WriteType::WithoutResponse)
            .await?;

        let read = self.peripheral.read(&self.auth).await?;

        if read != pin {
            return Err(ConnectionError::IncorrectPin);
        }

        Ok(())
    }

    pub async fn read_stdio(&mut self) -> Vec<u8> {
        self.peripheral
            .read(&self.rx_user)
            .await
            .unwrap()
    }
}

impl Connection for BluetoothConnection {
    async fn send_packet(&mut self, packet: impl Encode) -> Result<(), ConnectionError> {
        if !self.is_authenticated().await? {
            return Err(ConnectionError::AuthenticationRequired);
        }

        // Encode the packet
        let encoded = packet.encode()?;

        trace!("Sending packet: {:x?}", encoded);

        // Write the packet to the system tx characteristic.
        self.peripheral
            .write(&self.tx_system, &encoded, WriteType::WithoutResponse)
            .await?;

        Ok(())
    }

    async fn receive_packet<P: Decode>(&mut self, timeout: Duration) -> Result<P, ConnectionError> {
        todo!();
    }
}
