use std::{str::FromStr, time::Duration};

use tokio::{fs::File, io::AsyncWriteExt};
use vex_v5_serial::{
    commands::file::DownloadFile,
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    packets::{
        file::{FileTransferTarget, FileVendor},
        radio::{
            RadioChannel, SelectRadioChannelPacket, SelectRadioChannelPayload,
            SelectRadioChannelReplyPacket,
        },
    },
    string::FixedString,
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

    connection
        .packet_handshake::<SelectRadioChannelReplyPacket>(
            Duration::from_millis(500),
            10,
            SelectRadioChannelPacket::new(SelectRadioChannelPayload {
                channel: RadioChannel::Pit,
            }),
        )
        .await?
        .try_into_inner()
        .unwrap();

    let file = "slot_1.bin";

    // Download program file
    let download = connection
        .execute_command(DownloadFile {
            file_name: FixedString::from_str(file).unwrap(),
            size: 312340,
            vendor: FileVendor::User,
            target: Some(FileTransferTarget::Qspi),
            load_addr: 58720256,
            progress_callback: Some(Box::new(move |progress| {
                log::info!("{}: {:.2}%", file, progress);
            }) as Box<dyn FnMut(f32) + Send>),
        })
        .await?;

    let mut file = File::create_new(file).await?;
    file.write_all(&download).await?;

    Ok(())
}
