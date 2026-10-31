use std::{str::FromStr, time::Duration};

use macro_rules_attribute::apply;
use smol::{Timer, fs::File, io::AsyncWriteExt};
use vex_serial::{
    AsyncConnection,
    protocol::{
        FixedString,
        cdc2::file::{
            FileControlGroup, FileControlPacket, FileTransferTarget, FileVendor, RadioChannel,
        },
    },
    serial::{self, SerialError},
};
use vex_serial_util::file::download_file;

#[apply(smol_macros::main!)]
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
    let mut connection = devices[0].connect_async()?;

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

    Timer::after(Duration::from_millis(1000)).await;

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

    let mut file = File::create(file).await?;
    file.write_all(&download).await?;

    Ok(())
}
