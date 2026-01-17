use std::time::Duration;

use log::{error, info};
use tokio::time::sleep;
use vex_v5_serial::{
    Connection,
    protocol::{
        cdc::{ProductType, SystemVersionPacket},
        cdc2::controller::{CompetitionControlPacket, CompetitionMode},
    },
    serial::{self, SerialError},
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
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
        .handshake(SystemVersionPacket {}, Duration::from_millis(700), 5)
        .await?;

    match response.product_type {
        ProductType::V5Brain | ProductType::ExpBrain => {
            error!("You must be connected to the Brain over controller to use field control");
            return Ok(());
        }
        _ => {}
    }

    info!("Setting match mode to auto");
    connection
        .handshake(
            CompetitionControlPacket {
                mode: CompetitionMode::Autonomous,
                time: 0,
            },
            Duration::from_millis(500),
            10,
        )
        .await??;

    sleep(Duration::from_secs(2)).await;

    info!("Setting match mode to driver");
    connection
        .handshake(
            CompetitionControlPacket {
                mode: CompetitionMode::Driver,
                time: 2,
            },
            Duration::from_millis(500),
            10,
        )
        .await??;

    // 1 minute 45 seconds
    sleep(Duration::from_secs(2)).await;

    info!("Setting match mode to disabled");
    connection
        .handshake(
            CompetitionControlPacket {
                mode: CompetitionMode::Disabled,
                time: 4,
            },
            Duration::from_millis(500),
            10,
        )
        .await??;

    Ok(())
}
