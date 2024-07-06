//! Factory Control

use super::cdc2::{Cdc2CommandPacket, Cdc2ReplyPacket};
use crate::{
    array::Array,
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
};

pub struct FdtStatus {
    pub count: u8,
    pub files: Array<Fdt>,
}
impl Decode for FdtStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let count = u8::decode(&mut data)?;
        let entries = Array::decode_with_len(&mut data, count as _)?;
        Ok(Self {
            count,
            files: entries,
        })
    }
}

pub struct Fdt {
    pub index: u8,
    pub fdt_type: u8,
    pub status: u8,
    pub beta_version: u8,
    pub version: u16,
    pub boot_version: u16,
}
impl Decode for Fdt {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let index = u8::decode(&mut data)?;
        let fdt_type = u8::decode(&mut data)?;
        let status = u8::decode(&mut data)?;
        let beta_version = u8::decode(&mut data)?;
        let version = u16::decode(&mut data)?;
        let boot_version = u16::decode(&mut data)?;

        Ok(Self {
            index,
            fdt_type,
            status,
            beta_version,
            version,
            boot_version,
        })
    }
}

pub struct FactoryStatus {
    pub status: u8,
    pub percent: u8,
}
impl Decode for FactoryStatus {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let status = u8::decode(&mut data)?;
        let percent = u8::decode(&mut data)?;
        Ok(Self { status, percent })
    }
}

pub type GetFdtStatusPacket = Cdc2CommandPacket<86, 35, ()>;
pub type GetFdtStatusReplyPacket = Cdc2ReplyPacket<86, 35, FdtStatus>;

pub type GetFactoryStatusPacket = Cdc2CommandPacket<86, 241, ()>;
pub type GetFactoryStatusReplyPacket = Cdc2ReplyPacket<86, 241, FactoryStatus>;

pub type FactoryEnablePacket = Cdc2CommandPacket<86, 255, FactoryEnablePayload>;
pub type FactoryEnableReplyPacket = Cdc2CommandPacket<86, 255, ()>;

#[derive(Debug, Clone, Copy)]
pub struct FactoryEnablePayload(pub [u8; 4]);
impl Encode for FactoryEnablePayload {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.0.to_vec())
    }
}

impl Default for FactoryEnablePayload {
    fn default() -> Self {
        Self::new()
    }
}

impl FactoryEnablePayload {
    pub const FACTORY_ENABLE_BYTES: [u8; 4] = [77, 76, 75, 74];

    pub const fn new() -> Self {
        Self(Self::FACTORY_ENABLE_BYTES)
    }
}
