use std::time::Duration;

use vex_v5_serial::{
    commands::file::{ProgramData, UploadProgram},
    connection::{serial, Connection, ConnectionError},
    packets::{
        controller::{
            ControllerChannel, SwitchControllerChannelPacket, SwitchControllerChannelReplyPacket,
            SwitchcControllerChannelPayload,
        },
        file::FileExitAction,
    },
};

#[tokio::main]
async fn main() -> Result<(), ConnectionError> {
    // Initialize the logger
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
    let cold_bytes = include_bytes!("./basic.bin").to_vec();

    let callback_generator = |step| {
        Box::new(move |progress| {
            log::info!("{}: {:.2}%", step, progress);
        })
    };

    connection
        .packet_handshake::<SwitchControllerChannelReplyPacket>(
            Duration::from_millis(500),
            10,
            SwitchControllerChannelPacket::new(SwitchcControllerChannelPayload {
                unknown: 1,
                channel: ControllerChannel::Pit,
            }),
        )
        .await?
        .try_into_inner()
        .unwrap();

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
            after_upload: FileExitAction::RunProgram,
            ini_callback: Some(callback_generator("INI")),
            cold_callback: Some(callback_generator("Cold")),
            hot_callback: Some(callback_generator("Hot")),
        })
        .await?;

    Ok(())
}
