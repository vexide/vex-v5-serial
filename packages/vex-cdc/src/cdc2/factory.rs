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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryChallengePacket {}

impl Encode for FactoryChallengePacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryStatusPacket {}

impl Encode for FactoryStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryEnableReplyPacket {}

impl Decode for FactoryEnableReplyPacket {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {})
    }
}

//MARK: FactoryHwStatus
cdc2_pair!(
    FactoryHwStatusPacket => FactoryHwStatusReplyPacket,
    cmds::USER_CDC,
    ecmds::FACTORY_HW_STATUS
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryHwStatusPacket {}

impl Encode for FactoryHwStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryHwStatusReplyPacket {
    // total number of connected smart devices
    pub num_devices: u8,
    //number of full 11W motors (also counted as part of total connected)
    pub num_11w_motors: u8,
    //number of 5.5W/"ClassRoom" motors (also counted as part of total connected)
    pub num_55w_motors: u8,
    //this is not the actual radio device status, but contains
    //radio link status, mode (bt/vn3), and if one is connected.
    pub radio_status: u8,
    //the internal build version of the radio firmware
    pub radio_version: u8,
}

impl Decode for FactoryHwStatusReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            num_devices: u8::decode(data)?,
            num_11w_motors: u8::decode(data)?,
            num_55w_motors: u8::decode(data)?,
            radio_status: u8::decode(data)?,
            radio_version: u8::decode(data)?,
        })
    }
}

//MARK: FactoryOpctrStatus

cdc2_pair!(
    FactoryOpctrStatusPacket => FactoryOpctrStatusReplyPacket,
    cmds::USER_CDC,
    ecmds::FACTORY_OPCTR_STATUS
);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryOpctrStatusPacket {}

impl Encode for FactoryOpctrStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }

    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FactoryOpctrStatusReplyPacket {
    pub brain_id: u32,
    pub opctr_system: u32,
    pub opctr_user: u32,
    pub opctr_runs: u32,
}

impl Decode for FactoryOpctrStatusReplyPacket {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            brain_id: u32::decode(data)?,
            opctr_system: u32::decode(data)?,
            opctr_user: u32::decode(data)?,
            opctr_runs: u32::decode(data)?,
        })
    }
}
