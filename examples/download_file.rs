use std::{str::FromStr, time::Duration};

use tokio::{fs::File, io::AsyncWriteExt, time::sleep};
use vex_v5_serial::{
    Connection,
    commands::file::DownloadFile,
    protocol::{
        FixedString,
        cdc2::file::{
            FileControlGroup, FileControlPacket, FileControlReplyPacket, FileTransferTarget,
            FileVendor, RadioChannel,
        },
    },
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
        .await?
        .payload?;

    sleep(Duration::from_millis(1000)).await;

    let file = "slot_1.bin";

    // Download program file
    let download = connection
        .execute_command(DownloadFile {
            file_name: FixedString::from_str(file).unwrap(),
            size: 2000,
            vendor: FileVendor::User,
            target: FileTransferTarget::Qspi,
            address: 0x03800000,
            progress_callback: Some(Box::new(move |progress| {
                log::info!("{}: {:.2}%", file, progress);
            }) as Box<dyn FnMut(f32) + Send>),
        })
        .await?;

    let mut file = File::create_new(file).await?;
    file.write_all(&download).await?;

    Ok(())
}
