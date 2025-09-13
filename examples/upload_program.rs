use std::time::Duration;

use vex_v5_serial::{
    commands::file::{ProgramData, UploadProgram},
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    packets::file::{
        FileControlGroup, FileControlPacket, FileControlReplyPacket, FileExitAction, RadioChannel,
    },
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    // Initialize the logger
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;
    let program_data = include_bytes!("./basic.bin").to_vec();

    let callback_generator = |step| {
        Box::new(move |progress| {
            log::info!("{}: {:.2}%", step, progress);
        })
    };

    // Swap radio to download channel.
    //
    // This is a very naive approach to doing this for demonstration purposes.
    // `cargo-v5` has a more advanced polling-based implementation which is faster
    // and can be found here::
    // <https://github.com/vexide/cargo-v5/blob/main/src/connection.rs#L61>
    connection
        .handshake::<FileControlReplyPacket>(
            Duration::from_millis(500),
            10,
            FileControlPacket::new(FileControlGroup::Radio(RadioChannel::Download)),
        )
        .await?
        .payload?;

    // Upload program file
    connection
        .execute_command(UploadProgram {
            name: "quick".to_string(),
            description: "A basic vexide program".to_string(),
            icon: "USER029x.bmp".to_string(),
            program_type: "vexide".to_string(),
            slot: 4,
            data: ProgramData::Monolith(program_data),
            compress: true,
            after_upload: FileExitAction::RunProgram,
            ini_callback: Some(callback_generator("INI")),
            lib_callback: Some(callback_generator("LIB")),
            bin_callback: Some(callback_generator("BIN")),
        })
        .await?;

    Ok(())
}
