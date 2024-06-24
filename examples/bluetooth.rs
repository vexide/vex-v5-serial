use std::time::Duration;

use rustyline::DefaultEditor;
use vexv5_serial::{
    connection::{
        bluetooth::{self, BluetoothConnection},
        Connection,
    },
    packets::dash::{DashScreen, SelectDashPacket, SelectDashPayload},
};

#[tokio::main]
async fn main() {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Scan for 10 seconds, or until we find one device.
    let mut devices = bluetooth::find_devices(Duration::from_secs(10), Some(1))
        .await
        .unwrap()
        .into_iter();

    // Open a connection to the device
    let mut connection = BluetoothConnection::open(devices.nth(0).unwrap())
        .await
        .unwrap();

    if !connection.is_authenticated().await.unwrap() {
        connection.request_pin().await.unwrap();

        let mut editor = DefaultEditor::new().unwrap();
        let pin = editor.readline("Enter PIN: >> ").unwrap();

        let mut chars = pin.chars();

        connection
            .authenticate([
                chars.next().unwrap().to_digit(10).unwrap() as u8,
                chars.next().unwrap().to_digit(10).unwrap() as u8,
                chars.next().unwrap().to_digit(10).unwrap() as u8,
                chars.next().unwrap().to_digit(10).unwrap() as u8,
            ])
            .await
            .unwrap();
    }

    // Send a dash packet to test things out
    connection
        .send_packet(SelectDashPacket::new(SelectDashPayload {
            screen: DashScreen::ScaryConfiguration,
            port: 0,
        }))
        .await
        .unwrap();
}
