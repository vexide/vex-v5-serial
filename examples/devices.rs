use std::str::FromStr;

use log::info;
use vex_v5_serial::{
    connection::{
        serial::{self, SerialError},
        Connection,
    }, encode::Encode, packets::{
        device::{GetDeviceStatusPacket, GetDeviceStatusReplyPacket},
        file::{ExtensionType, FileType},
    }, string::FixedString
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    println!(
        "{:?}",
        FileType::new(FixedString::from_str("bin").unwrap(), ExtensionType::EncryptedBinary).encode()
    );
    Ok(())
}
