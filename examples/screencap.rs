use std::time::Duration;

use image::{GenericImageView, RgbImage};
use vexv5_serial::{
    commands::file::{DownloadFile, ScreenCapture},
    packets::{
        capture::{ScreenCapturePacket, ScreenCaptureReplyPacket},
        file::{FileDownloadTarget, FileVendor},
    },
    string::FixedLengthString,
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
    let vex_ports = vexv5_serial::connection::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    device
        .execute_command(ScreenCapture)
        .await
        .unwrap()
        .save("screencap.png")
        .unwrap();
}
