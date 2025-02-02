use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use super::file::FileVendor;
use crate::decode::SizedDecode;
use crate::string::FixedString;
use crate::{
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Slot {
    /// The number in the file icon: 'USER???x.bmp'.
    pub icon_number: u16,
    pub name_length: u8,
    pub name: String,
}
impl Decode for Slot {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let icon_number = u16::decode(&mut data)?;
        let name_length = u8::decode(&mut data)?;
        let name = String::sized_decode(&mut data, (name_length - 1) as _)?;

        Ok(Self {
            icon_number,
            name_length,
            name,
        })
    }
}

pub type GetProgramInfoPacket = Cdc2CommandPacket<86, 28, GetProgramInfoPayload>;
pub type GetProgramInfoReplyPacket = Cdc2ReplyPacket<86, 28, GetProgramInfoReplyPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GetProgramInfoPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    /// The bin file name.
    pub file_name: FixedString<23>,
}
impl Encode for GetProgramInfoPayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];

        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct GetProgramInfoReplyPayload {
    /// A zero-based slot number.
    pub slot: u8,

    /// A zero-based slot number, always same as Slot.
    pub requested_slot: u8,
}

pub type GetSlot1To4InfoPacket = Cdc2CommandPacket<86, 49, ()>;
pub type GetSlot1To4InfoReplyPacket = Cdc2CommandPacket<86, 49, SlotInfoPayload>;
pub type GetSlot5To8InfoPacket = Cdc2CommandPacket<86, 50, ()>;
pub type GetSlot5To8InfoReplyPacket = Cdc2CommandPacket<86, 50, SlotInfoPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SlotInfoPayload {
    /// Bit Mask.
    ///
    /// `flags & 2^(x - 1)` = Is slot x used
    pub flags: u8,

    /// Individual Slot Data
    pub slots: Vec<Slot>,
}
impl Decode for SlotInfoPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let flags = u8::decode(&mut data)?;
        let slots = Vec::sized_decode(&mut data, 4)?;

        Ok(Self { flags, slots })
    }
}
