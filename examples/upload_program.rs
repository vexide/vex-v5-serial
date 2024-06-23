use std::time::Duration;

use vexv5_serial::{
    commands::file::{ProgramData, UploadProgram},
    packets::file::FileExitAtion,
    connection::serial::find_devices,
};

#[tokio::main]
async fn main() {
    // Initialize the logger
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = find_devices().unwrap();

    // Open a connection to the device
    let mut connection = devices[0].open(Duration::from_secs(30)).unwrap();
    let cold_bytes = include_bytes!("./basic.bin").to_vec();

    // Upload program file
    connection
        .execute_command(UploadProgram {
            name: "quick".to_string(),
            description: "A basic vexide program".to_string(),
            icon: "USER029x.bmp".to_string(),
            program_type: "vexide".to_string(),
            slot: 4,
            data: ProgramData::Cold(cold_bytes),
            compress_program: true,
            after_upload: FileExitAtion::RunProgram,
        })
        .await
        .unwrap();
}
