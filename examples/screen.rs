use std::time::Duration;

use tokio::time::sleep;
use vexv5_serial::{commands::screen::{MockTap, OpenDashScreen, ScreenCapture}, packets::dash::DashScreen};

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
    let vex_ports = vexv5_serial::connection::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    device
        .execute_command(ScreenCapture)
        .await
        .unwrap()
        .save("screencap.png")
        .unwrap();

    device.execute_command(OpenDashScreen {
        dash: DashScreen::Home
    }).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    device.execute_command(MockTap {
        x: 300,
        y: 100,
    }).await.unwrap();
}
