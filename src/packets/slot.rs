use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use super::file::FileVendor;
use super::{Decode, DynamicVarLengthString, Encode, TerminatedFixedLengthString};

pub struct Slot {
    /// The number in the file icon: 'USER???x.bmp'.
    pub icon_number: u16,
    pub name_length: u8,
    pub name: DynamicVarLengthString,
}
impl Decode for Slot {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, super::DecodeError> {
        let mut data = data.into_iter();
        let icon_number = u16::decode(&mut data)?;
        let name_length = u8::decode(&mut data)?;
        let name = DynamicVarLengthString::decode_with_max_size(&mut data, (name_length - 1) as _)?;

        Ok(Self {
            icon_number,
            name_length,
            name,
        })
    }
}

pub type GetProgramSlotInfoPacket = Cdc2CommandPacket<0x56, 0x1c, GetProgramSlotInfoPayload>;
pub type GetLogCountReplyPacket = Cdc2ReplyPacket<0x56, 0x1c, GetProgramSlotInfoReplyPayload>;

pub struct GetProgramSlotInfoPayload {
    pub vendor: FileVendor,
    /// 0 = default. (RESEARCH NEEDED)
    pub option: u8,
    /// The bin file name.
    pub file_name: TerminatedFixedLengthString<23>,
}
impl Encode for GetProgramSlotInfoPayload {
    fn encode(&self) -> Result<Vec<u8>, super::EncodeError> {
        let mut encoded = vec![self.vendor as _, self.option];

        encoded.extend(self.file_name.encode()?);

        Ok(encoded)
    }
}

pub struct GetProgramSlotInfoReplyPayload {
    /// A zero-based slot number.
    pub slot: u8,

    /// A zero-based slot number, always same as Slot.
    pub requested_slot: u8,
}

pub type GetSlot1To4InfoPacket = Cdc2CommandPacket<0x56, 0x31, ()>;
pub type GetSlot1To4InfoReplyPacket = Cdc2CommandPacket<0x56, 0x31, SlotInfoPayload>;
pub type GetSlot5To8InfoPacket = Cdc2CommandPacket<0x56, 0x32, ()>;
pub type GetSlot5To8InfoReplyPacket = Cdc2CommandPacket<0x56, 0x32, SlotInfoPayload>;

pub struct SlotInfoPayload {
    /// Bit Mask.
    ///
    /// `flags & 2^(x - 1)` = Is slot x used
    pub flags: u8,

    /// Individual Slot Data
    pub slots: [Slot; 4],
}
impl Decode for SlotInfoPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, super::DecodeError> {
        let mut data = data.into_iter();
        let flags = u8::decode(&mut data)?;
        let slots = Decode::decode(&mut data)?;

        Ok(Self { flags, slots })
    }
}
