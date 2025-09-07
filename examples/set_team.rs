use std::time::Duration;

use vex_v5_serial::connection::serial::SerialError;
use vex_v5_serial::connection::{serial, Connection};
use vex_v5_serial::packets::system::{
    KeyValueLoadPacket, KeyValueLoadReplyPacket, KeyValueSavePacket, KeyValueSavePayload,
    KeyValueSaveReplyPacket,
};
use vex_v5_serial::string::FixedString;

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Trace,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Find all vex devices on the serial ports
    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    // Set the team number on the brain
    connection
        .send_packet(KeyValueSavePacket::new(KeyValueSavePayload {
            key: FixedString::new("teamnumber")?,
            value: FixedString::new(
                "vexide is number 1! vexide is number 1! vexide is number 1! vexide is number 1!",
            )?,
        }))
        .await?;
    connection
        .receive_packet::<KeyValueSaveReplyPacket>(Duration::from_millis(100))
        .await?;

    // Get the new team number and print it
    connection
        .send_packet(KeyValueLoadPacket::new(
            FixedString::new("teamnumber").unwrap(),
        ))
        .await?;
    let res = connection
        .receive_packet::<KeyValueLoadReplyPacket>(Duration::from_millis(100))
        .await?
        .try_into_inner()?;

    println!("{:?}", res);

    Ok(())
}
