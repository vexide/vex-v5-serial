use std::time::Duration;

use tokio::time::sleep;
use vexv5_serial::{
    commands::screen::{MockTap, OpenDashScreen, ScreenCapture},
    connection::{Connection, serial},
    packets::dash::DashScreen,
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

    // Find all vex devices on the serial ports
    let devices = serial::find_devices().unwrap();

    // Open a connection to the device
    let mut connection = devices[0].open(Duration::from_secs(30)).unwrap();

    connection
        .execute_command(ScreenCapture)
        .await
        .unwrap()
        .save("screencap.png")
        .unwrap();

    connection
        .execute_command(OpenDashScreen {
            dash: DashScreen::Home,
        })
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;

    connection
        .execute_command(MockTap { x: 300, y: 100 })
        .await
        .unwrap();
}
