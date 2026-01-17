use std::time::Duration;

use log::info;

use crate::Connection;

use vex_cdc::{
    FixedString,
    cdc2::{
        file::{FileTransferTarget, FileVendor},
        system::{
            DashScreen, DashSelectPacket, DashSelectPayload, DashSelectReplyPacket,
            DashTouchPacket, DashTouchPayload, DashTouchReplyPacket, ScreenCapturePacket,
            ScreenCapturePayload, ScreenCaptureReplyPacket,
        },
    },
};

use super::file::download_file;

pub async fn screen_capture<C: Connection + ?Sized>(
    connection: &mut C,
) -> Result<image::RgbImage, C::Error> {
    // Tell the brain we want to take a screenshot
    connection
        .handshake::<ScreenCaptureReplyPacket>(
            Duration::from_millis(100),
            5,
            ScreenCapturePacket::new(ScreenCapturePayload { layer: None }),
        )
        .await?;

    // Grab the image data
    let cap = download_file(
        connection,
        FixedString::new("screen".to_string()).unwrap(),
        512 * 272 * 4,
        FileVendor::Sys,
        FileTransferTarget::Cbuf,
        0,
        Some(|progress| {
            info!("Downloading screen: {:.2}%", progress)
        }),
    )
    .await?;

    let colors = cap
        .chunks(4)
        .filter_map(|p| {
            if p.len() == 4 {
                // little endian
                let color = [p[2], p[1], p[0]];
                Some(color)
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();

    let image = image::RgbImage::from_vec(512, 272, colors).unwrap();
    Ok(image::GenericImageView::view(&image, 0, 0, 480, 272).to_image())
}

pub async fn mock_touch<C: Connection + ?Sized>(
    connection: &mut C,
    x: u16,
    y: u16,
    pressed: bool,
) -> Result<(), C::Error> {
    connection
        .handshake::<DashTouchReplyPacket>(
            Duration::from_millis(100),
            5,
            DashTouchPacket::new(DashTouchPayload {
                x,
                y,
                pressing: if pressed { 1 } else { 0 },
            }),
        )
        .await?;
    Ok(())
}

pub async fn mock_tap<C: Connection + ?Sized>(
    connection: &mut C,
    x: u16,
    y: u16,
) -> Result<(), C::Error> {
    mock_touch(connection, x, y, true).await?;
    mock_touch(connection, x, y, false).await?;
    Ok(())
}

pub async fn open_dash_screen<C: Connection + ?Sized>(
    connection: &mut C,
    dash: DashScreen,
) -> Result<(), C::Error> {
    connection
        .handshake::<DashSelectReplyPacket>(
            Duration::from_millis(100),
            5,
            DashSelectPacket::new(DashSelectPayload {
                screen: dash,
                port: 0,
            }),
        )
        .await?;

    Ok(())
}