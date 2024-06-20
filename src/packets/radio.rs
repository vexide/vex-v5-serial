use super::{
    cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket},
    Decode,
};

pub struct RadioStatus {
    /// 0 = No controller, 4 = Controller connected (UNCONFIRMED)
    pub device: u8,
    /// From 0 to 100
    pub quality: u16,
    /// Always negative
    pub strength: i16,
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

pub type GetRadioStatusPacket = Cdc2CommandPacket<0x56, 0x26, ()>;
pub type GetRadioStatusReplyPacket = Cdc2ReplyPacket<0x56, 0x26, RadioStatus>;
