//! AIM-only remote display lifecycle, inspected in official VEXcom 1.0.2b2.
//! See docs/display-investigation.md in vexaim-cli for provenance/hardware gates.
//! BLE executes remote writes without returning their USB-style acknowledgments.
use super::{cdc2_command_size, frame_cdc2_command};
use crate::{Decode, DecodeError, DecodeErrorKind, Encode, cdc::cmds, cdc2_pair};

cdc2_pair!(RemoteDisplayPacket => RemoteDisplayReply, cmds::RC_CDC, 0x7f);

#[derive(Clone, Copy, Debug)]
pub struct RemoteDisplayPacket {
    pub start: bool,
}
impl Encode for RemoteDisplayPacket {
    fn size(&self) -> usize {
        cdc2_command_size(32)
    }
    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |payload| {
            payload.fill(0);
            payload[0] = 32;
            payload[1] = if self.start { 0x19 } else { 0x1a };
        });
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RemoteDisplayReply;
impl Decode for RemoteDisplayReply {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self)
    }
}

cdc2_pair!(RemoteStatusPacket => RemoteStatusReply, cmds::RC_CDC, 0x7e);
#[derive(Clone, Copy, Debug)]
pub struct RemoteStatusPacket;
impl Encode for RemoteStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }
    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RemoteStatusReply;
impl Decode for RemoteStatusReply {
    fn decode(_data: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self)
    }
}

// AIM's SYS_STATUS is seven bytes, unlike V5's version/status structure.
cdc2_pair!(ScreenStatusPacket => ScreenStatusReply, cmds::USER_CDC, 0x20);
#[derive(Clone, Copy, Debug)]
pub struct ScreenStatusPacket;
impl Encode for ScreenStatusPacket {
    fn size(&self) -> usize {
        cdc2_command_size(0)
    }
    fn encode(&self, data: &mut [u8]) {
        frame_cdc2_command(self, data, |_| {});
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ScreenStatusReply {
    pub screen_id: u8,
}
impl Decode for ScreenStatusReply {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        if data.len() != 7 {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedEnd));
        }
        let bytes = <[u8; 7]>::decode(data)?;
        Ok(Self {
            screen_id: bytes[3],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VEX_CRC16, cdc2::Cdc2Ack};
    use alloc::vec;
    fn encoded(packet: impl Encode) -> alloc::vec::Vec<u8> {
        let mut bytes = vec![0xa5; packet.size()];
        packet.encode(&mut bytes);
        bytes
    }
    #[test]
    fn official_remote_payload_and_crc_fixtures() {
        for (start, op, crc) in [(true, 0x19, [0x7d, 0x2c]), (false, 0x1a, [0xff, 0x76])] {
            let bytes = encoded(RemoteDisplayPacket { start });
            assert_eq!(&bytes[..7], &[0xc9, 0x36, 0xb8, 0x47, 0x57, 0x7f, 32]);
            assert_eq!(&bytes[7..9], &[32, op]);
            assert!(bytes[9..39].iter().all(|b| *b == 0));
            assert_eq!(&bytes[39..], &crc);
        }
        assert_eq!(
            &encoded(RemoteStatusPacket)[..7],
            &[0xc9, 0x36, 0xb8, 0x47, 0x57, 0x7e, 0]
        );
    }
    fn status() -> alloc::vec::Vec<u8> {
        let mut bytes = vec![0xaa, 0x55, 0x56, 11, 0x20, 0x76, 4, 6, 17, 20, 5, 9, 32];
        bytes.extend_from_slice(&VEX_CRC16.checksum(&bytes).to_be_bytes());
        bytes
    }
    #[test]
    fn validates_aim_status_family_crc_and_length() {
        let bytes = status();
        assert_eq!(
            Result::<ScreenStatusReply, Cdc2Ack>::decode(&mut bytes.as_slice())
                .unwrap()
                .unwrap()
                .screen_id,
            20
        );
        let mut bad = bytes.clone();
        bad[14] ^= 1;
        assert!(Result::<ScreenStatusReply, Cdc2Ack>::decode(&mut bad.as_slice()).is_err());
        let mut bad = bytes.clone();
        bad[2] = 0x57;
        assert!(Result::<ScreenStatusReply, Cdc2Ack>::decode(&mut bad.as_slice()).is_err());
        assert!(ScreenStatusReply::decode(&mut &[0; 6][..]).is_err());
    }
}
