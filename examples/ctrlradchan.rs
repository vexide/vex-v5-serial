use std::time::Duration;

use log::info;
use vex_v5_serial::{
    Connection,
    protocol::cdc2::controller::{ConfigureRadioPacket, ConfigureRadioPayload, ConfigureRadioReplyPacket, GetSmartfieldDataPacket, GetSmartfieldDataReplyPacket},
    serial::{self, SerialError},
};

#[tokio::main]
async fn main() -> Result<(), SerialError> {
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    let devices = serial::find_devices()?;

    // Open a connection to the device
    let mut connection = devices[0].connect(Duration::from_secs(30))?;

    // let status = connection
    //     .handshake::<ConfigureRadioReplyPacket>(
    //         Duration::from_millis(500),
    //         10,
    //         ConfigureRadioPacket::new(ConfigureRadioPayload{
    //             con_types: 0xFF,
    //             chan_type: 2,
    //             chan_num: 0, //select for me pls
    //             remote_ssn: 0xDEADC0DE,
    //             local_ssn: 0xC0DEC0DE,
    //         }),
    //     )
    //     .await?
    //     .payload?;

     let status = connection
        .handshake::<GetSmartfieldDataReplyPacket>(
            Duration::from_millis(500),
            10,
            GetSmartfieldDataPacket::new(()),
        )
        .await?
        .payload;

    println!("{:?}",status);

    Ok(())
}
