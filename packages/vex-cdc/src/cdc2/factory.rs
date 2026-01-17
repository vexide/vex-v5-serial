//! Factory control packets.

use crate::{
    Decode, DecodeError, Encode,
    cdc::cmds,
    cdc2::{cdc2_command_size, ecmds, frame_cdc2_command},
    cdc2_pair,
};

// MARK: FactoryChallenge

cdc2_pair!(
    FactoryChallengePacket => FactoryChallengeReplyPacket,
    cmds::USER_CDC,
    ecmds::FACTORY_CHAL
);

pub struct FactoryChallengePacket {}

impl Encode for FactoryChallengePacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

pub struct FactoryChallengeReplyPacket {
    pub challenge_bytes: [u8; 16],
}

impl Decode for FactoryChallengeReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            challenge_bytes: Decode::decode(data)?,
        })
    }
}

// MARK: FactoryResponse

cdc2_pair!(
    FactoryResponsePacket => FactoryResponseReplyPacket,
    cmds::USER_CDC,
    ecmds::FACTORY_RESP
);

pub struct FactoryResponsePacket {
    pub response_bytes: [u8; 16],
}

impl Encode for FactoryResponsePacket {
    fn size(&self) -> usize {
        cdc2_command_size(16)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.response_bytes.encode(data);
        });
    }
}

pub struct FactoryResponseReplyPacket {}

impl Decode for FactoryResponseReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

// MARK: FactoryStatus

cdc2_pair!(
    FactoryStatusPacket => FactoryStatusReplyPacket,
    cmds::USER_CDC,
    ecmds::FACTORY_STATUS
);

pub struct FactoryStatusPacket {}

impl Encode for FactoryStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

pub struct FactoryStatusReplyPacket {
    pub status: u8,
    pub percent: u8,
}

impl Decode for FactoryStatusReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            status: Decode::decode(data)?,
            percent: Decode::decode(data)?,
        })
    }
}

// MARK: FactoryEnable

cdc2_pair!(
    FactoryEnablePacket => FactoryEnableReplyPacket,
    cmds::USER_CDC,
    ecmds::FACTORY_EBL
);

pub struct FactoryEnablePacket {
    pub magic: [u8; 4],
}

impl FactoryEnablePacket {
    pub const MAGIC: [u8; 4] = [0x4D, 0x4C, 0x4B, 0x4A];
}

impl Encode for FactoryEnablePacket {
    fn size(&self) -> usize {
        cdc2_command_size(4)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |data| {
            self.magic.encode(data);
        });
    }
}

pub struct FactoryEnableReplyPacket {}

impl Decode for FactoryEnableReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}
