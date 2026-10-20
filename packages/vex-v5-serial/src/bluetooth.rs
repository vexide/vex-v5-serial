use std::time::{Duration, Instant};
use std::{
    collections::{HashSet, VecDeque},
    future::Future,
    pin::Pin,
};

use btleplug::api::{
    Central, CentralEvent, CentralState, Characteristic, Manager as _, Peripheral as _, ScanFilter,
    ValueNotification, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use futures_util::{FutureExt, Stream, StreamExt};
use log::{debug, error, trace, warn};
use thiserror::Error;
use uuid::Uuid;

use crate::trim_packets;

use vex_cdc::{Decode, DecodeError, Encode, FixedStringSizeError, cdc2::Cdc2Ack};

use super::{Connection, ConnectionType, RawPacket};

/// The BLE GATT Service that V5 Brains provide
pub const V5_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);
/// AIM advertises this service; V5_SERVICE is available only after connecting.
pub const AIM_ADVERTISED_SERVICE: Uuid = Uuid::from_u128(0x00000670_0000_1000_8000_00805f9b34fb);

const MAX_BUFFER_BYTES: usize = 65536;
const MAX_PENDING_PACKETS: usize = 64;

async fn with_timeout<T>(
    duration: Duration,
    future: impl Future<Output = Result<T, BluetoothError>>,
) -> Result<T, BluetoothError> {
    futures_util::select! {
        result = Box::pin(future).fuse() => result,
        _ = FutureExt::fuse(async_io::Timer::after(duration)) => Err(BluetoothError::Timeout),
    }
}

fn matches_service(services: &[Uuid]) -> bool {
    services.contains(&V5_SERVICE) || services.contains(&AIM_ADVERTISED_SERVICE)
}

fn auth_state(bytes: &[u8]) -> Result<bool, BluetoothError> {
    let value: [u8; 4] = bytes
        .try_into()
        .map_err(|_| BluetoothError::InvalidAuthentication)?;
    if u32::from_be_bytes(value) == UNPAIRED_MAGIC {
        Ok(false)
    } else if value.iter().all(|digit| *digit <= 9) {
        Ok(true)
    } else {
        Err(BluetoothError::InvalidAuthentication)
    }
}

fn write_limit(mtu: u16) -> Result<usize, BluetoothError> {
    match mtu {
        0 => Ok(20), // Some backends cannot report a negotiated MTU.
        1..=22 => Err(BluetoothError::InvalidMtu),
        _ => Ok(usize::from(mtu - 3).min(BluetoothConnection::MAX_PACKET_SIZE)),
    }
}

#[derive(Default)]
struct NotificationBuffers {
    user: VecDeque<u8>,
    system: Vec<u8>,
    packets: Vec<RawPacket>,
}

impl NotificationBuffers {
    fn receive(&mut self, notification: ValueNotification) -> Result<(), BluetoothError> {
        if notification.uuid == CHARACTERISTIC_USER_TX {
            if self.user.len() + notification.value.len() > MAX_BUFFER_BYTES {
                return Err(BluetoothError::BufferOverflow);
            }
            self.user.extend(notification.value);
        } else if notification.uuid == CHARACTERISTIC_SYSTEM_TX {
            if self.system.len() + notification.value.len() > MAX_BUFFER_BYTES {
                return Err(BluetoothError::BufferOverflow);
            }
            self.system.extend(notification.value);
            // Notifications can split or coalesce CDC frames. Keep incomplete frames.
            while self.system.len() >= 4 {
                if !self.system.starts_with(&[0xaa, 0x55]) {
                    self.system.remove(0);
                    continue;
                }
                let first = self.system[3];
                let (header, payload) = if first & 0x80 != 0 {
                    if self.system.len() < 5 {
                        break;
                    }
                    (
                        5,
                        usize::from(u16::from_be_bytes([first & 0x7f, self.system[4]])),
                    )
                } else {
                    (4, usize::from(first))
                };
                let length = header + payload;
                if self.system.len() < length {
                    break;
                }
                trim_packets(&mut self.packets);
                if self.packets.len() >= MAX_PENDING_PACKETS {
                    return Err(BluetoothError::BufferOverflow);
                }
                self.packets
                    .push(RawPacket::new(self.system.drain(..length).collect()));
            }
        }
        Ok(())
    }

    fn read_user(&mut self, output: &mut [u8]) -> usize {
        let count = output.len().min(self.user.len());
        for byte in &mut output[..count] {
            *byte = self.user.pop_front().unwrap();
        }
        count
    }
}

/// System port GATT characteristic
pub const CHARACTERISTIC_SYSTEM_TX: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306); // WRITE_WITHOUT_RESPONSE | NOTIFY | INDICATE
pub const CHARACTERISTIC_SYSTEM_RX: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13f5); // WRITE_WITHOUT_RESPONSE | WRITE | NOTIFY

/// User port GATT characteristic
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

/// Discover VEX BLE candidates (verify AIM model before running AIM-specific code).
/// Call from a Tokio runtime, as required by btleplug's platform drivers.
pub async fn find_devices(
    scan_time: Duration,
    max_device_count: Option<usize>,
) -> Result<Vec<BluetoothDevice>, BluetoothError> {
    if max_device_count == Some(0) {
        return Ok(Vec::new());
    }
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    if adapters.is_empty() {
        return Err(BluetoothError::NoBluetoothAdapter);
    }
    let mut selected = None;
    for adapter in adapters {
        if !matches!(adapter.adapter_state().await?, CentralState::PoweredOff) {
            selected = Some(adapter);
            break;
        }
    }
    let adapter = selected.ok_or(BluetoothError::AdapterPoweredOff)?;

    // Our bluetooth adapter will give us an event stream that can tell us when
    // a device is discovered. We can use this to get information on when a scan
    // has found a device.
    let mut events = adapter.events().await?;

    // List of devices that we'll add to during discovery.
    let mut devices = Vec::<BluetoothDevice>::new();
    let mut seen = HashSet::new();

    // Post-filter: BlueZ may merge other clients' filters, and AIM does not advertise V5_SERVICE.
    let scan_start_time = Instant::now();
    adapter.start_scan(ScanFilter::default()).await?;

    // Listen for events. When the adapter indicates that a device has been discovered,
    // we'll ensure that the peripheral is correct and add it to our device list.
    let result = with_timeout(scan_time, async {
        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                    let peripheral = adapter.peripheral(&id).await?;

                    if let Some(properties) = peripheral.properties().await? {
                        if matches_service(&properties.services) && seen.insert(peripheral.id()) {
                            debug!("Found VEX BLE candidate at {}", peripheral.address());

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
        Ok(())
    })
    .await;
    // Cleanup runs on success, deadline and discovery failure; a silent event stream cannot hang.
    let cleanup = with_timeout(Duration::from_secs(2), async {
        Ok(adapter.stop_scan().await?)
    })
    .await;
    if !matches!(result, Err(BluetoothError::Timeout)) {
        result?;
    }
    cleanup?;

    debug!(
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

    notification_stream: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    buffers: NotificationBuffers,
}

impl BluetoothConnection {
    pub const MAX_PACKET_SIZE: usize = 244;

    pub async fn open(device: BluetoothDevice) -> Result<Self, BluetoothError> {
        let peripheral = device.0;

        let result = with_timeout(Duration::from_secs(15), async {
            if !peripheral.is_connected().await? {
                peripheral.connect().await?;
            } else {
                warn!("Peripheral already connected?");
            }
            Self::configure(peripheral.clone()).await
        })
        .await;
        if result.is_err() {
            let _ = with_timeout(Duration::from_secs(2), async {
                Ok(peripheral.disconnect().await?)
            })
            .await;
        }
        result
    }

    async fn configure(peripheral: Peripheral) -> Result<Self, BluetoothError> {
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
            notification_stream: peripheral.notifications().await?,
            peripheral,
            system_tx: system_tx.ok_or(BluetoothError::MissingCharacteristic)?,
            system_rx: system_rx.ok_or(BluetoothError::MissingCharacteristic)?,
            user_tx: user_tx.ok_or(BluetoothError::MissingCharacteristic)?,
            user_rx: user_rx.ok_or(BluetoothError::MissingCharacteristic)?,
            pairing: pairing.ok_or(BluetoothError::MissingCharacteristic)?,
            buffers: NotificationBuffers::default(),
        };

        connection
            .peripheral
            .subscribe(&connection.system_tx)
            .await?;
        connection.peripheral.subscribe(&connection.user_tx).await?;

        Ok(connection)
    }

    pub async fn is_paired(&self) -> Result<bool, BluetoothError> {
        with_timeout(Duration::from_secs(5), async {
            let auth_bytes = self.peripheral.read(&self.pairing).await?;
            auth_state(&auth_bytes)
        })
        .await
    }

    pub async fn request_pairing(&mut self) -> Result<(), BluetoothError> {
        with_timeout(Duration::from_secs(5), async {
            self.peripheral
                .write(
                    &self.pairing,
                    &[0xFF, 0xFF, 0xFF, 0xFF],
                    WriteType::WithResponse,
                )
                .await?;

            Ok(())
        })
        .await
    }

    pub async fn authenticate_pairing(&mut self, pin: [u8; 4]) -> Result<(), BluetoothError> {
        if pin.iter().any(|digit| *digit > 9) {
            return Err(BluetoothError::InvalidAuthentication);
        }
        with_timeout(Duration::from_secs(5), async {
            self.peripheral
                .write(&self.pairing, &pin, WriteType::WithResponse)
                .await?;

            let read = self.peripheral.read(&self.pairing).await?;

            if read != pin {
                return Err(BluetoothError::IncorrectPin);
            }

            Ok(())
        })
        .await
    }

    async fn receive_notification(&mut self) -> Result<(), BluetoothError> {
        let Some(notification) = self.notification_stream.next().await else {
            return Err(BluetoothError::NoResponse);
        };

        self.buffers.receive(notification)
    }

    /// Explicit cleanup; callers should invoke this on both successful and failed commands.
    pub async fn close(&mut self) -> Result<(), BluetoothError> {
        with_timeout(Duration::from_secs(2), async {
            Ok(self.peripheral.disconnect().await?)
        })
        .await
    }

    async fn write_chunks(
        &self,
        characteristic: &Characteristic,
        bytes: &[u8],
    ) -> Result<(), BluetoothError> {
        let limit = write_limit(self.peripheral.mtu())?;
        for chunk in bytes.chunks(limit) {
            with_timeout(Duration::from_secs(5), async {
                Ok(self
                    .peripheral
                    .write(characteristic, chunk, WriteType::WithoutResponse)
                    .await?)
            })
            .await?;
            async_io::Timer::after(Duration::from_millis(10)).await;
        }
        Ok(())
    }
}

impl Connection for BluetoothConnection {
    type Error = BluetoothError;

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Bluetooth
    }

    async fn send(&mut self, packet: impl Encode) -> Result<(), BluetoothError> {
        if !self.is_paired().await? {
            return Err(BluetoothError::PairingRequired);
        }

        // Encode the packet
        let mut encoded = vec![0; packet.size()];
        packet.encode(&mut encoded);

        trace!("sending {} system bytes", encoded.len());

        // Write the packet to the system rx characteristic.
        self.write_chunks(&self.system_rx, &encoded).await?;

        Ok(())
    }

    async fn recv<P: Decode>(&mut self, timeout: Duration) -> Result<P, BluetoothError> {
        // Return an error if the right packet is not received within the timeout
        futures_util::select! {
            result = async {
                loop {
                    for packet in self.buffers.packets.iter_mut() {
                        match packet.decode_and_use::<P>() {
                            Some(Ok(decoded)) => {
                                trim_packets(&mut self.buffers.packets);
                                return Ok(decoded);
                            }
                            Some(Err(e)) => {
                                error!("Failed to decode packet with valid header: {}", e);
                                packet.used = true;
                                return Err(BluetoothError::DecodeError(e));
                            }
                            None => {}
                        }
                    }

                    trim_packets(&mut self.buffers.packets);
                    self.receive_notification().await?;
                }
            }.fuse() => result,
            _ = FutureExt::fuse(async_io::Timer::after(timeout)) => Err(BluetoothError::Timeout)
        }
    }

    async fn read_user(&mut self, buf: &mut [u8]) -> Result<usize, BluetoothError> {
        if buf.is_empty() {
            return Ok(0);
        }
        with_timeout(Duration::from_millis(100), async {
            while self.buffers.user.is_empty() {
                self.receive_notification().await?;
            }
            Ok(self.buffers.read_user(buf))
        })
        .await
    }

    async fn write_user(&mut self, buf: &[u8]) -> Result<usize, BluetoothError> {
        if !self.is_paired().await? {
            return Err(BluetoothError::PairingRequired);
        }
        self.write_chunks(&self.user_rx, buf).await?;
        Ok(buf.len())
    }
}

#[derive(Error, Debug)]
pub enum BluetoothError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Packet decoding error: {0}")]
    DecodeError(#[from] DecodeError),
    #[error("Packet timeout")]
    Timeout,
    #[error("NACK received: {0:?}")]
    Nack(#[from] Cdc2Ack),
    #[error("Bluetooth error: {0}")]
    Btleplug(#[from] btleplug::Error),
    #[error("No response received over bluetooth")]
    NoResponse,
    #[error("No Bluetooth Adapter Found")]
    NoBluetoothAdapter,
    #[error("Bluetooth adapter is powered off; enable Bluetooth in system settings")]
    AdapterPoweredOff,
    #[error("Malformed authentication response or PIN; expected four numeric digit bytes")]
    InvalidAuthentication,
    #[error("Invalid negotiated Bluetooth MTU")]
    InvalidMtu,
    #[error("Bluetooth receive buffer overflow; output is incomplete")]
    BufferOverflow,
    #[error("Expected a Bluetooth characteristic that didn't exist")]
    MissingCharacteristic,
    #[error("Authentication PIN code was incorrect")]
    IncorrectPin,
    #[error("Pairing is required")]
    PairingRequired,
    #[error(transparent)]
    FixedStringSizeError(#[from] FixedStringSizeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(uuid: Uuid, bytes: &[u8]) -> ValueNotification {
        ValueNotification {
            uuid,
            service_uuid: V5_SERVICE,
            value: bytes.to_vec(),
        }
    }

    #[test]
    fn discovers_actual_aim_advertisement_without_v5_service() {
        assert!(matches_service(&[AIM_ADVERTISED_SERVICE]));
        assert!(matches_service(&[V5_SERVICE]));
        assert!(!matches_service(&[Uuid::nil()]));
        assert!(!matches_service(&[]));
    }

    #[test]
    fn auth_rejects_malformed_responses_without_panicking() {
        for bytes in [&[][..], &[0, 1, 2][..], &[0, 1, 2, 3, 4][..], &[255; 4][..]] {
            assert!(matches!(
                auth_state(bytes),
                Err(BluetoothError::InvalidAuthentication)
            ));
        }
        assert!(!auth_state(&UNPAIRED_MAGIC.to_be_bytes()).unwrap());
        assert!(auth_state(&[0, 1, 2, 3]).unwrap());
        assert!(auth_state(&[0, 0, 0, 0]).unwrap());
    }

    #[test]
    fn mtu_caps_payload_and_never_exceeds_small_mtu() {
        assert_eq!(write_limit(0).unwrap(), 20);
        assert_eq!(write_limit(23).unwrap(), 20);
        assert_eq!(write_limit(100).unwrap(), 97);
        assert_eq!(write_limit(256).unwrap(), 244);
        assert_eq!(write_limit(517).unwrap(), 244);
        assert!(write_limit(3).is_err());
    }

    #[test]
    fn partial_user_reads_preserve_every_byte_across_system_traffic() {
        let mut buffers = NotificationBuffers::default();
        buffers
            .receive(notification(CHARACTERISTIC_USER_TX, b"hello"))
            .unwrap();
        buffers
            .receive(notification(
                CHARACTERISTIC_SYSTEM_TX,
                &[0xaa, 0x55, 0x56, 1, 7],
            ))
            .unwrap();
        buffers
            .receive(notification(CHARACTERISTIC_USER_TX, b" world"))
            .unwrap();
        let mut first = [0; 3];
        assert_eq!(buffers.read_user(&mut first), 3);
        assert_eq!(&first, b"hel");
        let mut remainder = [0; 20];
        assert_eq!(buffers.read_user(&mut remainder), 8);
        assert_eq!(&remainder[..8], b"lo world");
        assert_eq!(buffers.packets.len(), 1);
        assert_eq!(buffers.packets[0].bytes, [0xaa, 0x55, 0x56, 1, 7]);
    }

    #[test]
    fn system_frames_survive_every_notification_boundary_and_coalescing() {
        let mut frame = vec![0xaa, 0x55, 0x56, 0x80, 130];
        frame.extend([42; 130]);
        let second = [0xaa, 0x55, 0x56, 2, 1, 2];
        for split in 0..=frame.len() {
            let mut buffers = NotificationBuffers::default();
            buffers
                .receive(notification(CHARACTERISTIC_SYSTEM_TX, &frame[..split]))
                .unwrap();
            buffers
                .receive(notification(CHARACTERISTIC_USER_TX, b"keep me"))
                .unwrap();
            let rest = [&frame[split..], &second].concat();
            buffers
                .receive(notification(CHARACTERISTIC_SYSTEM_TX, &rest))
                .unwrap();
            assert_eq!(buffers.packets.len(), 2, "split {split}");
            assert_eq!(buffers.packets[0].bytes, frame);
            assert_eq!(buffers.packets[1].bytes, second);
            assert_eq!(buffers.user.len(), 7);
        }
    }

    #[test]
    fn resynchronizes_noise_without_losing_partial_header() {
        let mut buffers = NotificationBuffers::default();
        buffers
            .receive(notification(CHARACTERISTIC_SYSTEM_TX, &[0, 0, 0, 0xaa]))
            .unwrap();
        buffers
            .receive(notification(CHARACTERISTIC_SYSTEM_TX, &[0x55, 0x56, 1, 9]))
            .unwrap();
        assert_eq!(buffers.packets.len(), 1);
        assert_eq!(buffers.packets[0].bytes, [0xaa, 0x55, 0x56, 1, 9]);
    }

    #[test]
    fn queue_overflow_fails_explicitly_instead_of_dropping_output() {
        let mut buffers = NotificationBuffers::default();
        buffers
            .receive(notification(
                CHARACTERISTIC_USER_TX,
                &vec![0; MAX_BUFFER_BYTES],
            ))
            .unwrap();
        assert!(matches!(
            buffers.receive(notification(CHARACTERISTIC_USER_TX, b"x")),
            Err(BluetoothError::BufferOverflow)
        ));
        assert_eq!(buffers.user.len(), MAX_BUFFER_BYTES);
        for _ in 0..MAX_PENDING_PACKETS {
            buffers
                .receive(notification(
                    CHARACTERISTIC_SYSTEM_TX,
                    &[0xaa, 0x55, 0x56, 1, 9],
                ))
                .unwrap();
        }
        assert!(matches!(
            buffers.receive(notification(
                CHARACTERISTIC_SYSTEM_TX,
                &[0xaa, 0x55, 0x56, 1, 9]
            )),
            Err(BluetoothError::BufferOverflow)
        ));
    }

    #[test]
    fn silent_future_has_a_real_deadline() {
        let result = async_io::block_on(with_timeout(
            Duration::from_millis(10),
            futures_util::future::pending::<Result<(), BluetoothError>>(),
        ));
        assert!(matches!(result, Err(BluetoothError::Timeout)));
    }
}
