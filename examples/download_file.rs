use std::{str::FromStr, time::Duration};

use tokio::{fs::File, io::AsyncWriteExt, time::sleep};
use vex_v5_serial::{
    Connection,
    commands::file::download_file,
    protocol::{
        FixedString,
        cdc2::file::{
            FileControlGroup, FileControlPacket, FileTransferTarget,
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
        .await??;

    sleep(Duration::from_millis(1000)).await;

    let file = "slot_1.bin";

    // Download program file
    let download = download_file(
        &mut connection,
        FixedString::from_str(file).unwrap(),
        2000,
        FileVendor::User,
        FileTransferTarget::Qspi,
        0x03800000,
        Some(move |progress| {
            log::info!("{}: {:.2}%", file, progress);
        }),
    )
    .await?;

    let mut file = File::create_new(file).await?;
    file.write_all(&download).await?;

    Ok(())
}
