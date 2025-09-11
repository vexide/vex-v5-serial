use std::time::Duration;

use log::info;
use vex_v5_serial::{
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    packets::system::{SystemVersionPacket, SystemVersionReplyPacket},
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    let response = connection
        .handshake::<SystemVersionReplyPacket>(
            Duration::from_millis(700),
            5,
            SystemVersionPacket::new(()),
        )
        .await?;

    info!("{:?}", response.payload.product_type);

    Ok(())
}
