use std::time::{Duration, Instant};

use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use log::{debug, info, trace, warn};
use thiserror::Error;
use tokio::select;
use tokio::time::sleep;
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::connection::trim_packets;
use crate::decode::{Decode, DecodeError};
use crate::encode::{Encode, EncodeError};
use crate::packets::cdc2::Cdc2Ack;

use super::{Connection, ConnectionType, RawPacket};

/// The BLE GATT Service that V5 Brains provide
pub const V5_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

/// User port GATT characteristic
pub const CHARACTERISTIC_SYSTEM_TX: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
pub const CHARACTERISTIC_SYSTEM_RX: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13f5); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

/// System port GATT characteristic
pub const CHARACTERISTIC_USER_TX: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
pub const CHARACTERISTIC_USER_RX: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1326); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIF

/// PIN authentication characteristic
pub const CHARACTERISTIC_PAIRING: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5); // READ | WRITE_WITHOUT_RESPONSE | WRITE

pub const UNPAIRED_MAGIC: u32 = 0xdeadface;

#[derive(Debug, Clone)]
pub struct BluetoothDevice(pub Peripheral);

impl BluetoothDevice {
    pub async fn connect(&self) -> Result<BluetoothConnection, BluetoothError> {
        BluetoothConnection::open(self.clone()).await
    }
}

/// Discover and locate bluetooth-compatible V5 peripherals.
pub async fn find_devices(
    scan_time: Duration,
    max_device_count: Option<usize>,
) -> Result<Vec<BluetoothDevice>, BluetoothError> {
    // Create a new bluetooth device manager.
    let manager = Manager::new().await?;

    // Use the first adapter we can find.
    let adapter = if let Some(adapter) = manager.adapters().await?.into_iter().next() {
        adapter
    } else {
        // No bluetooth adapters were found.
        return Err(BluetoothError::NoBluetoothAdapter);
    };

    // Our bluetooth adapter will give us an event stream that can tell us when
    // a device is discovered. We can use this to get information on when a scan
    // has found a device.
    let mut events = adapter.events().await?;

    // List of devices that we'll add to during discovery.
    let mut devices = Vec::<BluetoothDevice>::new();

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

                        devices.push(BluetoothDevice(peripheral));

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
    pub peripheral: Peripheral,
    pub system_tx: Characteristic,
    pub system_rx: Characteristic,
    pub user_tx: Characteristic,
    pub user_rx: Characteristic,
    pub pairing: Characteristic,

    incoming_packets: Vec<RawPacket>,
}

impl BluetoothConnection {
    pub const MAX_PACKET_SIZE: usize = 244;

    pub async fn open(device: BluetoothDevice) -> Result<Self, BluetoothError> {
        let peripheral = device.0;

        if !peripheral.is_connected().await? {
            peripheral.connect().await?;
        } else {
            warn!("Peripheral already connected?");
        }

        peripheral.discover_services().await?;

        let mut system_tx: Option<Characteristic> = None;
        let mut system_rx: Option<Characteristic> = None;
        let mut user_tx: Option<Characteristic> = None;
        let mut user_rx: Option<Characteristic> = None;
        let mut pairing: Option<Characteristic> = None;

        for characteric in peripheral.characteristics() {
            match characteric.uuid {
                CHARACTERISTIC_SYSTEM_TX => {
                    system_tx = Some(characteric);
                }
                CHARACTERISTIC_SYSTEM_RX => {
                    system_rx = Some(characteric);
                }
                CHARACTERISTIC_USER_TX => {
                    user_tx = Some(characteric);
                }
                CHARACTERISTIC_USER_RX => {
                    user_rx = Some(characteric);
                }
                CHARACTERISTIC_PAIRING => {
                    pairing = Some(characteric);
                }
                _ => {}
            }
        }

        let connection = Self {
            peripheral,
            system_tx: system_tx.ok_or(BluetoothError::MissingCharacteristic)?,
            system_rx: system_rx.ok_or(BluetoothError::MissingCharacteristic)?,
            user_tx: user_tx.ok_or(BluetoothError::MissingCharacteristic)?,
            user_rx: user_rx.ok_or(BluetoothError::MissingCharacteristic)?,
            pairing: pairing.ok_or(BluetoothError::MissingCharacteristic)?,

            incoming_packets: Vec::new(),
        };

        connection
            .peripheral
            .subscribe(&connection.system_tx)
            .await?;
        connection.peripheral.subscribe(&connection.user_tx).await?;

        Ok(connection)
    }

    pub async fn is_paired(&self) -> Result<bool, BluetoothError> {
        let auth_bytes = self.peripheral.read(&self.pairing).await?;

        Ok(u32::from_be_bytes(auth_bytes[0..4].try_into().unwrap()) != UNPAIRED_MAGIC)
    }

    pub async fn request_pairing(&mut self) -> Result<(), BluetoothError> {
        self.peripheral
            .write(
                &self.pairing,
                &[0xFF, 0xFF, 0xFF, 0xFF],
                WriteType::WithoutResponse,
            )
            .await?;

        Ok(())
    }

    pub async fn authenticate_pairing(&mut self, pin: [u8; 4]) -> Result<(), BluetoothError> {
        self.peripheral
            .write(&self.pairing, &pin, WriteType::WithoutResponse)
            .await?;

        let read = self.peripheral.read(&self.pairing).await?;

        if read != pin {
            return Err(BluetoothError::IncorrectPin);
        }

        Ok(())
    }

    async fn receive_one_packet(&mut self) -> Result<(), BluetoothError> {
        //TODO: get notifications and store it rather than creating it every time this method is called
        let mut notifs = self.peripheral.notifications().await?;

        loop {
            let Some(notification) = notifs.next().await else {
                return Err(BluetoothError::NoResponse);
            };

            if notification.uuid == CHARACTERISTIC_SYSTEM_TX {
                let data = notification.value;
                debug!("Received packet: {:x?}", data);
                let packet = RawPacket::new(data);
                self.incoming_packets.push(packet);
                break;
            }
        }

        Ok(())
    }
}

impl Connection for BluetoothConnection {
    type Error = BluetoothError;

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Bluetooth
    }

    async fn send_packet(&mut self, packet: impl Encode) -> Result<(), BluetoothError> {
        if !self.is_paired().await? {
            return Err(BluetoothError::PairingRequired);
        }

        // Encode the packet
        let encoded = packet.encode()?;

        trace!("Sending packet: {:x?}", encoded);

        // Write the packet to the system rx characteristic.
        self.peripheral
            .write(&self.system_rx, &encoded, WriteType::WithoutResponse)
            .await?;

        Ok(())
    }

    async fn receive_packet<P: Decode>(&mut self, timeout: Duration) -> Result<P, BluetoothError> {
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
            _ = sleep(timeout) => Err(BluetoothError::Timeout)
        }
    }

    async fn read_user(&mut self, _buf: &mut [u8]) -> Result<usize, BluetoothError> {
        todo!();
    }

    async fn write_user(&mut self, _buf: &[u8]) -> Result<usize, BluetoothError> {
        todo!();
    }
}

#[derive(Error, Debug)]
pub enum BluetoothError {
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
    #[error("Bluetooth Error")]
    Btleplug(#[from] btleplug::Error),
    #[error("No response received over bluetooth")]
    NoResponse,
    #[error("No Bluetooth Adapter Found")]
    NoBluetoothAdapter,
    #[error("Expected a Bluetooth characteristic that didn't exist")]
    MissingCharacteristic,
    #[error("Authentication PIN code was incorrect")]
    IncorrectPin,
    #[error("Pairing is required")]
    PairingRequired,
}
