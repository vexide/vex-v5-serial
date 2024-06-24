use std::time::Duration;

use rustyline::DefaultEditor;
use vex_v5_serial::{
    connection::{bluetooth, Connection, ConnectionError},
    packets::dash::{DashScreen, SelectDashPacket, SelectDashPayload},
};

#[tokio::main]
async fn main() -> Result<(), ConnectionError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Scan for 10 seconds, or until we find one device.
    let devices = bluetooth::find_devices(Duration::from_secs(10), Some(1)).await?;

    // Open a connection to the device
    let mut connection = devices[0].connect().await?;

    if !connection.is_paired().await? {
        connection.request_pairing().await?;

        let mut editor = DefaultEditor::new().unwrap();
        let pin = editor.readline("Enter PIN: >> ").unwrap();

        let mut chars = pin.chars();

        connection
            .authenticate_pairing([
                chars.next().unwrap().to_digit(10).unwrap() as u8,
                chars.next().unwrap().to_digit(10).unwrap() as u8,
                chars.next().unwrap().to_digit(10).unwrap() as u8,
                chars.next().unwrap().to_digit(10).unwrap() as u8,
            ])
            .await?;
    }

    // Send a dash packet to test things out
    connection
        .send_packet(SelectDashPacket::new(SelectDashPayload {
            screen: DashScreen::ScaryConfiguration,
            port: 0,
        }))
        .await?;

    Ok(())
}
