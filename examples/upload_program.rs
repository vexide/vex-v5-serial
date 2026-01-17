use std::time::Duration;

use vex_v5_serial::{
    Connection,
    commands::file::{ProgramData, upload_program},
    protocol::cdc2::file::{FileControlGroup, FileControlPacket, FileExitAction, RadioChannel},
    serial::{self, SerialError},
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
        move |progress| {
            log::info!("{}: {:.2}%", step, progress);
        }
    };

    // Swap radio to download channel.
    //
    // This is a very naive approach to doing this for demonstration purposes.
    // `cargo-v5` has a more advanced polling-based implementation which is faster
    // and can be found here::
    // <https://github.com/vexide/cargo-v5/blob/main/src/connection.rs#L61>
    connection
        .handshake(
            FileControlPacket {
                group: FileControlGroup::Radio(RadioChannel::Download),
            },
            Duration::from_millis(500),
            10,
        )
        .await??;

    // Upload program file
    upload_program(
        &mut connection,
        4,
        "quick",
        "A basic vexide program",
        "vexide",
        "USER029x.bmp",
        true,
        ProgramData::Monolith(program_data),
        FileExitAction::RunProgram,
        Some(callback_generator("INI")),
        Some(callback_generator("LIB")),
        Some(callback_generator("BIN")),
    )
    .await?;

    Ok(())
}
