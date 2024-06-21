use std::time::Duration;

use image::{GenericImageView, RgbImage};
use vexv5_serial::{
    commands::file::DownloadFile,
    packets::{
        capture::{ScreenCapturePacket, ScreenCaptureReplyPacket},
        file::{FileDownloadTarget, FileVendor},
    },
    string::FixedLengthString,
};

#[tokio::main]
async fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    device
        .send_packet(ScreenCapturePacket::new(()))
        .await
        .unwrap();
    device
        .recieve_packet::<ScreenCaptureReplyPacket>(Duration::from_millis(100))
        .await
        .unwrap();
    // Take a screenshot
    let cap = device
        .execute_command(DownloadFile {
            filename: FixedLengthString::new("screen".to_string()).unwrap(),
            filetype: FixedLengthString::new("".to_string()).unwrap(),
            vendor: FileVendor::Sys,
            target: Some(FileDownloadTarget::Cbuf),
            load_addr: 0,
            size: 512 * 272 * 4,
            progress_callback: Some(Box::new(|progress| {
                if progress != 100.0 {
                    print!("\x1B[sDownloading screencap: {progress:.2}%\x1B[u")
                } else {
                    println!("\x1B[sDownloading screencap: {progress:.2}%")
                }
            })),
        })
        .await
        .unwrap();

    let colors = cap
        .chunks(4)
        .filter_map(|p| {
            if p.len() == 4 {
                // little endian
                let color = [p[2], p[1], p[0]];
                Some(color)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    let image = RgbImage::from_vec(512, 272, colors).unwrap();
    image.view(0, 0, 480, 272).to_image().save("screencap.png").unwrap();
}
