use crate::encode::{Encode, EncodeError};

use super::{
    cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
    Decode,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RadioStatus {
    /// 0 = No controller, 4 = Controller connected (UNCONFIRMED)
    pub device: u8,
    /// From 0 to 100
    pub quality: u16,
    /// Probably RSSI (UNCONFIRMED)
    pub strength: i16,
    /// 5 = download, 31 = pit
    pub channel: i8,
    /// Latency between controller and brain (UNCONFIRMED)
    pub timeslot: i8,
}
impl Decode for RadioStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, super::DecodeError> {
        let mut data = data.into_iter();
        let device = u8::decode(&mut data)?;
        let quality = u16::decode(&mut data)?;
        let strength = i16::decode(&mut data)?;
        let channel = i8::decode(&mut data)?;
        let timeslot = i8::decode(&mut data)?;
        Ok(Self {
            device,
            quality,
            strength,
            channel,
            timeslot,
        })
    }
}

pub type GetRadioStatusPacket = Cdc2CommandPacket<86, 38, ()>;
pub type GetRadioStatusReplyPacket = Cdc2ReplyPacket<86, 38, RadioStatus>;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RadioChannel {
    // NOTE: There's probably a secret third channel for matches, but that's not known.
    /// Used when controlling the robot outside of a competition match.
    Pit = 0x00,

    /// Used when wirelessly uploading or downloading data to/from the V5 Brain.
    ///
    /// Higher radio bandwidth for file transfer purposes.
    Download = 0x01,
}
impl Encode for RadioChannel {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![*self as u8])
    }
}
pub type SelectRadioChannelPacket = Cdc2CommandPacket<86, 16, SelectRadioChannelPayload>;
pub type SelectRadioChannelReplyPacket = Cdc2ReplyPacket<86, 16, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SelectRadioChannelPayload {
    pub channel: RadioChannel,
}
impl Encode for SelectRadioChannelPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = Vec::new();
        // pros-cli keeps this byte at 1, which presumably specifies the radio file control group
        encoded.push(0x01);
        encoded.extend(self.channel.encode()?);
        Ok(encoded)
    }
}
