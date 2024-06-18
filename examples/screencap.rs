use vexv5_serial::{
    commands::file::DownloadFile,
    v5::{FileTransferTarget, FileTransferType, FileTransferVID},
};

#[tokio::main]
async fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    // Take a screenshot
    let cap = device
        .execute_command(DownloadFile {
            filename: "screen".into(),
            filetype: FileTransferType::Other([0; 3]),
            vendor: FileTransferVID::System,
            target: Some(FileTransferTarget::Cbuf),
            load_addr: 0,
            size: 512,
            progress_callback: Some(Box::new(|progress| {
                println!("Downloading screencap: {progress:.2}")
            })),
        })
        .await
        .unwrap();

    println!("Downloaded screencap: {:?}", cap);
    println!("Downloaded screencap: {:?}", cap.len());
    let colors = cap.chunks(4).map(|p| {
        if p.len() == 4 {
            let bytes = [p[0], p[1], p[2], p[3]];
            Some(u32::from_le_bytes(bytes))
        } else {
            None
        }
    }).filter(|p| p.is_some());
    println!("Colors: {:x?}", colors.collect::<Vec<_>>());
}
