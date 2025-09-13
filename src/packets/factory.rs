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
    packets::cdc::CdcReplyPacket,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryStatus {
    pub status: u8,
    pub percent: u8,
}
impl Decode for FactoryStatus {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let status = u8::decode(data)?;
        let percent = u8::decode(data)?;
        Ok(Self { status, percent })
    }
}

pub type FactoryChallengePacket = Cdc2CommandPacket<USER_CDC, FACTORY_CHAL, ()>;
pub type FactoryChallengeReplyPacket = CdcReplyPacket<USER_CDC, [u8; 16]>;

pub type FactoryResponsePacket = Cdc2CommandPacket<USER_CDC, FACTORY_RESP, [u8; 16]>;
pub type FactoryResponseReplyPacket = Cdc2ReplyPacket<USER_CDC, FACTORY_RESP, ()>;

pub type FactoryStatusPacket = Cdc2CommandPacket<USER_CDC, FACTORY_STATUS, ()>;
pub type FactoryStatusReplyPacket = Cdc2ReplyPacket<USER_CDC, FACTORY_STATUS, FactoryStatus>;

pub type FactoryEnablePacket = Cdc2CommandPacket<USER_CDC, FACTORY_EBL, [u8; 4]>;
pub type FactoryEnableReplyPacket = Cdc2ReplyPacket<USER_CDC, FACTORY_EBL, ()>;

impl FactoryEnablePacket {
    pub const MAGIC: [u8; 4] = [0x4D, 0x4C, 0x4B, 0x4A];
}
