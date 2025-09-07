use std::time::Duration;

use log::{error, info};
use tokio::time::sleep;
use vex_v5_serial::{
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    packets::{
        controller::{
            CompetitionControlPacket, CompetitionControlPayload, CompetitionControlReplyPacket,
            MatchMode,
        },
        system::{SystemVersionPacket, SystemVersionReplyPacket},
    },
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
        .packet_handshake::<SystemVersionReplyPacket>(
            Duration::from_millis(700),
            5,
            SystemVersionPacket::new(()),
        )
        .await?;

    match response.payload.product_type {
        vex_v5_serial::packets::system::ProductType::Brain => {
            error!("You must be connected to the Brain over controller to use field control");
            return Ok(());
        }
        vex_v5_serial::packets::system::ProductType::Controller => {}
    }

    info!("Setting match mode to auto");
    connection
        .packet_handshake::<CompetitionControlReplyPacket>(
            Duration::from_millis(500),
            10,
            CompetitionControlPacket::new(CompetitionControlPayload {
                match_mode: MatchMode::Auto,
                match_time: 0,
            }),
        )
        .await?;

    sleep(Duration::from_secs(2)).await;

    info!("Setting match mode to driver");
    connection
        .packet_handshake::<CompetitionControlReplyPacket>(
            Duration::from_millis(500),
            10,
            CompetitionControlPacket::new(CompetitionControlPayload {
                match_mode: MatchMode::Driver,
                match_time: 2,
            }),
        )
        .await?;

    // 1 minute 45 seconds
    sleep(Duration::from_secs(2)).await;

    info!("Setting match mode to disabled");
    connection
        .packet_handshake::<CompetitionControlReplyPacket>(
            Duration::from_millis(500),
            10,
            CompetitionControlPacket::new(CompetitionControlPayload {
                match_mode: MatchMode::Disabled,
                match_time: 4,
            }),
        )
        .await?;

    Ok(())
}
