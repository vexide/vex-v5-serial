//! Simple CDC packets.

use crate::{
    decode::{Decode, DecodeError, DecodeErrorKind},
    encode::{Encode, MessageEncoder},
    varint::VarU16,
    version::Version,
};

use super::{COMMAND_HEADER, REPLY_HEADER};

/// CDC packet opcodes.
///
/// These are the byte values identifying the different CDC commands.
/// This module is non-exhaustive.
pub mod cmds {
    pub const ACK: u8 = 0x33;
    pub const QUERY_1: u8 = 0x21;
    pub const USER_CDC: u8 = 0x56;
    pub const CON_CDC: u8 = 0x58;
    pub const SYSTEM_VERSION: u8 = 0xA4;
    pub const EEPROM_ERASE: u8 = 0x31;
    pub const USER_ENTER: u8 = 0x60;
    pub const USER_CATALOG: u8 = 0x61;
    pub const FLASH_ERASE: u8 = 0x63;
    pub const FLASH_WRITE: u8 = 0x64;
    pub const FLASH_READ: u8 = 0x65;
    pub const USER_EXIT: u8 = 0x66;
    pub const USER_PLAY: u8 = 0x67;
    pub const USER_STOP: u8 = 0x68;
    pub const COMPONENT_GET: u8 = 0x69;
    pub const USER_SLOT_GET: u8 = 0x78;
    pub const USER_SLOT_SET: u8 = 0x79;
    pub const BRAIN_NAME_GET: u8 = 0x44;
}

use bitflags::bitflags;
use cmds::{QUERY_1, SYSTEM_VERSION};

/// CDC (Simple) command packet.
///
/// A device-bound message containing a command identifier and an encoded payload.
/// Each packet begins with a 4-byte [`COMMAND_HEADER`], followed by the opcode,
/// and then an optional length-prefixed payload.
///
/// The payload type `P` must implement [`Encode`].
///
/// # Encoding
///
/// | Field     | Size   | Description |
/// |-----------|--------|-------------|
/// | `header`  | 4      | Must be [`COMMAND_HEADER`]. |
/// | `cmd`     | 1      | A [CDC command opcode](crate::cdc::cmds) indicating the type of packet. |
/// | `size`    | 1–2    | Size of `payload` encoded as a [`VarU16`]. |
/// | `payload` | n      | Encoded payload. |
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CdcCommandPacket<const CMD: u8, P: Encode> {
    payload: P,
}

impl<const CMD: u8, P: Encode> CdcCommandPacket<CMD, P> {
    /// Header used for device-bound VEX CDC packets.
    pub const HEADER: [u8; 4] = COMMAND_HEADER;

    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self { payload }
    }
}

impl<const CMD: u8, P: Encode> Encode for CdcCommandPacket<CMD, P> {
    fn size(&self) -> usize {
        let payload_size = self.payload.size();

        5 + if payload_size == 0 {
            0
        } else if payload_size > (u8::MAX >> 1) as _ {
            2
        } else {
            1
        } + payload_size
    }

    fn encode(&self, data: &mut [u8]) {
        Self::HEADER.encode(data);
        data[4] = CMD;

        let payload_size = self.payload.size();

        // We only encode the payload size if there is a payload
        if payload_size > 0 {
            let mut enc = MessageEncoder::new_with_position(data, 5);

            enc.write(&VarU16::new(payload_size as u16));
            enc.write(&self.payload);
        }
    }
}

/// CDC (Simple) reply packet.
///
/// A host-bound packet sent in response to a [`CdcCommandPacket`].  
/// Each reply consists of a 2-byte [`REPLY_HEADER`], the echoed command ID,
/// a variable-width length field, and the decoded payload.
///
/// The payload type `P` must implement [`Decode`].
///
/// # Encoding
///
/// | Field     | Size   | Description |
/// |-----------|--------|-------------|
/// | `header`  | 2      | Must be [`REPLY_HEADER`]. |
/// | `cmd`     | 1      | A [CDC command opcode](crate::cdc::cmds) indicating the type of command being replied to. |
/// | `size`    | 1–2    | Size of `payload` encoded as a [`VarU16`]. |
/// | `payload` | n      | Encoded payload. |
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CdcReplyPacket<const CMD: u8, P: Decode> {
    /// Packet Payload Size
    pub size: u16,

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    pub payload: P,
}

impl<const CMD: u8, P: Decode> CdcReplyPacket<CMD, P> {
    /// Header used for host-bound VEX CDC packets.
    pub const HEADER: [u8; 2] = REPLY_HEADER;
}

impl<const CMD: u8, P: Decode> Decode for CdcReplyPacket<CMD, P> {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        if <[u8; 2]>::decode(data)? != Self::HEADER {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::InvalidHeader));
        }

        let cmd = u8::decode(data)?;
        if cmd != CMD {
            return Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                name: "cmd",
                value: cmd,
                expected: &[CMD],
            }));
        }

        let payload_size = VarU16::decode(data)?.into_inner();
        let payload = P::decode(data)?;

        Ok(Self {
            size: payload_size,
            payload,
        })
    }
}

pub type SystemVersionPacket = CdcCommandPacket<SYSTEM_VERSION, ()>;
pub type SystemVersionReplyPacket = CdcReplyPacket<SYSTEM_VERSION, SystemVersionReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SystemVersionReplyPayload {
    pub version: Version,
    pub product_type: ProductType,
    pub flags: ProductFlags,
}
impl Decode for SystemVersionReplyPayload {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let version = Version::decode(data)?;
        let product_type = ProductType::decode(data)?;
        let flags = ProductFlags::from_bits_truncate(u8::decode(data)?);

        Ok(Self {
            version,
            product_type,
            flags,
        })
    }
}

pub type Query1Packet = CdcCommandPacket<QUERY_1, ()>;
pub type Query1ReplyPacket = CdcReplyPacket<QUERY_1, Query1ReplyPayload>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Query1ReplyPayload {
    pub version_1: u32,
    pub version_2: u32,

    /// 0xFF = QSPI, 0 = NOT sdcard, other = sdcard (returns devcfg.MULTIBOOT_ADDR)
    pub boot_source: u8,

    /// Number of times this packet has been replied to.
    pub count: u8,
}

impl Decode for Query1ReplyPayload {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let version_1 = u32::decode(data)?;
        let version_2 = u32::decode(data)?;
        let boot_source = u8::decode(data)?;
        let count = u8::decode(data)?;

        Ok(Self {
            version_1,
            version_2,
            boot_source,
            count,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u16)]
pub enum ProductType {
    Brain = 0x10,
    Controller = 0x11,
}
impl Decode for ProductType {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let data = <[u8; 2]>::decode(data)?;

        match data[1] {
            0x10 => Ok(Self::Brain),
            0x11 => Ok(Self::Controller),
            v => Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                name: "ProductType",
                value: v,
                expected: &[0x10, 0x11],
            })),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct ProductFlags: u8 {
        /// Bit 1 is set when the controller is connected over a cable to the V5 Brain
        const CONNECTED_CABLE = 1 << 0; // From testing, this appears to be how it works.

        /// Bit 2 is set when the controller is connected over VEXLink to the V5 Brain.
        const CONNECTED_WIRELESS = 1 << 1;
    }
}
