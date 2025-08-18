use std::{str::FromStr, time::Duration};

use tokio::{fs::File, io::AsyncWriteExt, time::sleep};
use vex_v5_serial::{
    commands::file::DownloadFile,
    connection::{
        serial::{self, SerialError},
        Connection,
    },
    packets::file::{
        FileControlGroup, FileControlPacket, FileControlReplyPacket, FileTransferTarget,
        FileVendor, RadioChannel,
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

    // Swap radio to download channel.
    //
    // This is a very naive approach to doing this for demonstration purposes.
    // `cargo-v5` has a more advanced polling-based implementation which is faster
    // and can be found here::
    // <https://github.com/vexide/cargo-v5/blob/main/src/connection.rs#L61>
    connection
        .packet_handshake::<FileControlReplyPacket>(
            Duration::from_millis(500),
            10,
            FileControlPacket::new(FileControlGroup::Radio(RadioChannel::Download)),
        )
        .await?
        .try_into_inner()
        .unwrap();

    sleep(Duration::from_millis(1000)).await;

    let file = "slot_3.bin";

    // Download program file
    let download = connection
        .execute_command(DownloadFile {
            file_name: FixedString::from_str(file).unwrap(),
            size: 2000,
            vendor: FileVendor::User,
            target: FileTransferTarget::Qspi,
            load_addr: 0x03800000,
            progress_callback: Some(Box::new(move |progress| {
                log::info!("{}: {:.2}%", file, progress);
            }) as Box<dyn FnMut(f32) + Send>),
        })
        .await?;

    let mut file = File::create_new(file).await?;
    file.write_all(&download).await?;

    Ok(())
}
