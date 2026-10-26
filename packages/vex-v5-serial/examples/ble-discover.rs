//! Stationary backend smoke test: discover VEX candidates without pairing or writing GATT values.
use std::time::Duration;

use btleplug::api::Peripheral as _;
use vex_v5_serial::bluetooth;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let devices = bluetooth::find_devices(Duration::from_secs(8), None).await?;
            for device in &devices {
                let properties = device.0.properties().await?;
                println!(
                    "{}\t{}",
                    device.0.id(),
                    properties.and_then(|p| p.local_name).unwrap_or_default()
                );
            }
            println!(
                "{} VEX BLE candidate(s); authentication not attempted",
                devices.len()
            );
            Ok(())
        })
}
