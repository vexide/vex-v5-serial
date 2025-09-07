//! Factory Control

use super::{
    cdc::cmds::USER_CDC,
    cdc2::{
        ecmds::{FACTORY_CHAL, FACTORY_EBL, FACTORY_RESP, FACTORY_STATUS},
        Cdc2CommandPacket, Cdc2ReplyPacket,
    },
};
use crate::{
    decode::{Decode, DecodeError},
    encode::{Encode, EncodeError},
    packets::cdc::CdcReplyPacket,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

pub type FactoryChallengePacket = Cdc2CommandPacket<USER_CDC, FACTORY_CHAL, ()>;
pub type FactoryChallengeReplyPacket = CdcReplyPacket<USER_CDC, FactoryChallengeReplyPayload>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FactoryChallengeReplyPayload {
    pub data: [u8; 16],
}

impl Decode for FactoryChallengeReplyPayload {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        Ok(Self {
            data: Decode::decode(data)?,
        })
    }
}

pub type FactoryResponsePacket = Cdc2CommandPacket<USER_CDC, FACTORY_RESP, [u8; 16]>;
pub type FactoryResponseReplyPacket = Cdc2ReplyPacket<USER_CDC, FACTORY_RESP, ()>;

pub type FactoryStatusPacket = Cdc2CommandPacket<USER_CDC, FACTORY_STATUS, ()>;
pub type FactoryStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, FACTORY_STATUS, FactoryStatus>;

pub type FactoryEnablePacket = Cdc2CommandPacket<USER_CDC, FACTORY_EBL, FactoryEnablePayload>;
pub type FactoryEnableReplyPacket = Cdc2ReplyPacket<USER_CDC, FACTORY_EBL, ()>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
    pub const MAGIC: [u8; 4] = [0x4D, 0x4C, 0x4B, 0x4A];

    pub const fn new() -> Self {
        Self(Self::MAGIC)
    }
}
