use std::time::Duration;

use tokio::time::sleep;
use vex_v5_serial::{
    commands::screen::{MockTap, OpenDashScreen, ScreenCapture},
    connection::{serial, Connection, ConnectionError},
    packets::dash::DashScreen,
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

    // Find all vex devices on the serial ports
    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    connection
        .execute_command(ScreenCapture)
        .await?
        .save("screencap.png")
        .unwrap();

    connection
        .execute_command(OpenDashScreen {
            dash: DashScreen::Home,
        })
        .await?;

    sleep(Duration::from_millis(50)).await;

    connection
        .execute_command(MockTap { x: 300, y: 100 })
        .await?;

    Ok(())
}
