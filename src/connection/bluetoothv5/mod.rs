use std::time::Duration;

use bluest::{Adapter, AdvertisingDevice, Characteristic, Service, Uuid};

use log::debug;
use tokio_stream::StreamExt;

use super::DeviceError;

/// The BLE GATT Service that V5 Brains provide
const GATT_SERVICE: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13d5);

/// The unknown GATT characteristic
const GATT_UNKNOWN: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1306);

/// The user port GATT characteristic
const GATT_USER: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb1316);

/// The system port GATT characteristic
const GATT_SYSTEM: Uuid = Uuid::from_u128(0x08590f7e_db05_467e_8757_72f6faeb13e5);

/// Represents a brain connected over bluetooth
#[derive(Clone, Debug)]
pub struct BluetoothBrain {
    adapter: Adapter,
    system_char: Option<Characteristic>,
    user_char: Option<Characteristic>,
    service: Option<Service>,
    device: AdvertisingDevice,
}

impl BluetoothBrain {
    pub fn new(adapter: Adapter, device: AdvertisingDevice) -> BluetoothBrain {
        Self {
            adapter,
            system_char: None,
            user_char: None,
            service: None,
            device,
        }
    }

    /// Connects self to .ok_or(DeviceError::NotConnected)the brain
    pub async fn connect(&mut self) -> Result<(), DeviceError> {
        // Create the adapter
        //self.adapter = Some(
        //    Adapter::default().await.ok_or(
        //        DeviceError::NoBluetoothAdapter
        //    )?
        //);

        // Wait for the adapter to be available
        self.adapter.wait_available().await?;

        // For some reason we need a little delay in here
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect to the device
        self.adapter.connect_device(&self.device.device).await?;

        // And here too
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get all services on the brain
        let services = self.device.device.discover_services().await?;

        // Find the vex service
        self.service = Some(
            services
                .iter()
                .find(|v| v.uuid() == GATT_SYSTEM)
                .ok_or(DeviceError::InvalidDevice)?
                .clone(),
        );
        if let Some(service) = &self.service {
            // Get all characteristics of this service
            let chars = service.discover_characteristics().await?;

            // Find the system characteristic
            self.system_char = Some(
                chars
                    .iter()
                    .find(|v| v.uuid() == GATT_SYSTEM)
                    .ok_or(DeviceError::InvalidDevice)?
                    .clone(),
            );
            // Find the user characteristic
            self.user_char = Some(
                chars
                    .iter()
                    .find(|v| v.uuid() == GATT_USER)
                    .ok_or(DeviceError::InvalidDevice)?
                    .clone(),
            );
        } else {
            return Err(DeviceError::InvalidDevice);
        }

        Ok(())
    }

    /// Handshakes with the device, telling it we have connected
    pub async fn handshake(&self) -> Result<(), DeviceError> {
        // Read data from the system characteristic,
        // making sure that it equals 0xdeadface (big endian)
        let data = self.read_system().await?;

        // If there are not four bytes, then error
        if data.len() != 4 {
            return Err(DeviceError::InvalidMagic);
        }

        // Parse the bytes into a big endian u32
        let magic = u32::from_be_bytes(data.try_into().unwrap());

        // If the magic number is not 0xdeadface, then it is an invalid device
        if magic != 0xdeadface {
            return Err(DeviceError::InvalidMagic);
        }

        debug!("{magic:x}");

        Ok(())
    }

    /// Writes to the system port
    pub async fn write_system(&self, buf: &[u8]) -> Result<(), DeviceError> {
        if let Some(system) = &self.system_char {
            Ok(system.write(buf).await?)
        } else {
            Err(DeviceError::NotConnected)
        }
    }

    /// Reads from the system port
    pub async fn read_system(&self) -> Result<Vec<u8>, DeviceError> {
        if let Some(system) = &self.system_char {
            Ok(system.read().await?)
        } else {
            Err(DeviceError::NotConnected)
        }
    }

    /// Disconnects self from the brain
    pub async fn disconnect(&self) -> Result<(), DeviceError> {
        // Disconnect the device
        self.adapter.disconnect_device(&self.device.device).await?;

        Ok(())
    }
}

/// Discovers all V5 devices that are advertising over bluetooth.
/// By default it scans for 5 seconds, but this can be configured
pub async fn scan_for_v5_devices(
    timeout: Option<Duration>,
) -> Result<Vec<BluetoothBrain>, DeviceError> {
    // If timeout is None, then default to five seconds
    let timeout = timeout.unwrap_or_else(|| Duration::new(5, 0));

    // Get the adapter and wait for it to be available
    let adapter = Adapter::default()
        .await
        .ok_or(DeviceError::NoBluetoothAdapter)?;
    adapter.wait_available().await?;

    // Create the GATT UUID
    let service: bluest::Uuid = GATT_SERVICE;
    let service = &[service];

    // Start scanning
    let scan_stream = adapter.scan(service).await?;

    // Set a timeout
    let timeout_stream = scan_stream.timeout(timeout);
    tokio::pin!(timeout_stream);

    // Find the current time
    let time = std::time::SystemTime::now();

    let mut devices = Vec::<BluetoothBrain>::new();

    // Find each device
    while let Ok(Some(discovered_device)) = timeout_stream.try_next().await {
        devices.push(BluetoothBrain::new(adapter.clone(), discovered_device));
        // If over timeout has passed, then break
        if time.elapsed().unwrap() >= timeout {
            break;
        }
    }

    // These are our brains
    Ok(devices)
}
