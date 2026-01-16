//! Extended CDC packets.

use core::fmt::Debug;
use thiserror::Error;

use crate::{
    cdc::{CdcCommand, CdcReply},
    crc::{VEX_CRC16, decode_crc16},
    decode::{Decode, DecodeError, DecodeErrorKind},
    encode::Encode,
    varint::VarU16,
};

pub mod controller;
pub mod factory;
pub mod file;
pub mod system;

/// CDC2 (Extended) Command Packet
///
/// This type encodes a host-to-device command packet used in the extended CDC2 protocol. The payload type `P`
/// must implement [`Encode`].
///
/// All CDC2 commands have a corresponding [`Cdc2ReplyPacket`] from the device, even if the reply contains no
/// payload. For example, a [`SystemFlagsPacket`] command corresponds to a [`SystemFlagsReplyPacket`] reply.
/// See [`Cdc2ReplyPacket`] for more info on packet acknowledgement.
///
/// [`SystemFlagsPacket`]: system::SystemFlagsPacket
/// [`SystemFlagsReplyPacket`]: system::SystemFlagsReplyPacket
///
/// # Encoding
///
/// This is an extension of the standard [`CdcCommandPacket`](crate::cdc::CdcCommandPacket) encoding that adds:
///
/// - An extended command opcode ([`ecmd`](ecmds)).
/// - A CRC16 checksum covering the entire packet (including header).
///
/// | Field     | Size   | Description |
/// |-----------|--------|-------------|
/// | `header`  | 4      | Must be [`COMMAND_HEADER`]. |
/// | `cmd`     | 1      | A [CDC command opcode](crate::cdc::cmds), typically [`USER_CDC`](crate::cdc::cmds::USER_CDC) (for a brain) or [`CON_CDC`](crate::cdc::cmds::CON_CDC) (for a controller). |
/// | `ecmd`    | 1      | A [CDC2 extended command opcode](ecmds). |
/// | `size`    | 1–2    | Number of remaining bytes in the packet (starting at `payload` through `crc16`), encoded as a [`VarU16`]. |
/// | `payload` | n      | Encoded payload. |
/// | `crc16`   | 2      | CRC16 checksum of the whole packet, computed with the [`VEX_CRC16`] algorithm. |
pub trait Cdc2Command: CdcCommand {
    const ECMD: u8;
}

pub(crate) const fn cdc2_command_size(payload_size: usize) -> usize {
    let frame_size = 8 + if payload_size > (u8::MAX >> 1) as _ {
        2
    } else {
        1
    };

    frame_size + payload_size
}

pub(crate) fn frame_cdc2_command<C: Cdc2Command>(
    packet: &C,
    data: &mut [u8],
    payload_fn: impl FnOnce(&mut [u8]),
) {
    C::HEADER.encode(data);
    data[4] = C::CMD;
    data[5] = C::ECMD;
    
    let command_size = packet.size();
    let frame_size = 8 + if (command_size - 6) > ((u8::MAX >> 1) as usize) {
        2
    } else {
        1
    };
    let payload_size = command_size - frame_size;
    
    VarU16::new(payload_size as u16).encode(&mut data[6..]);

    // Encode payload
    payload_fn(&mut data[(frame_size - 2)..(command_size - 2)]);

    // The CRC16 checksum is of the whole encoded packet, meaning we need
    // to also include the header bytes.
    VEX_CRC16
        .checksum(&data[0..(command_size - 2)])
        .to_be()
        .encode(&mut data[(command_size - 2)..]);
}

/// CDC2 (Extended) Reply Packet
///
/// This type decodes a device-to-host reply packet used in the extended CDC2 protocol. The payload type `P` must
/// implement [`Decode`].
///
/// All CDC2 replies are sent in response to a corresponding [`Cdc2CommandPacket`] from the host. For example,
/// a [`SystemFlagsPacket`] command corresponds to a [`SystemFlagsReplyPacket`] reply.
///
/// [`SystemFlagsPacket`]: system::SystemFlagsPacket
/// [`SystemFlagsReplyPacket`]: system::SystemFlagsReplyPacket
///
/// Reply packets encode an *acknowledgement code* in the form of a [`Cdc2Ack`], which will either acknowledge
/// the command as valid and successful ([`Cdc2Ack::Ack`]) or return a negative acknowledgement if there was a
/// problem processing or servicing the command.
///
/// # Encoding
///
/// This is an extension of the standard [`CdcReplyPacket`](crate::cdc::CdcReplyPacket) encoding that adds:
///
/// - An extended command opcode ([`ecmd`](ecmds)).
/// - A packet acknowledgement code ([`Cdc2Ack`]).
/// - A CRC16 checksum covering the entire packet (including header).
///
/// | Field     | Size   | Description |
/// |-----------|--------|-------------|
/// | `header`  | 2      | Must be [`REPLY_HEADER`]. |
/// | `cmd`     | 1      | A [CDC command opcode](crate::cdc::cmds), typically [`USER_CDC`](crate::cdc::cmds::USER_CDC) (for a brain) or [`CON_CDC`](crate::cdc::cmds::CON_CDC) (for a controller). |
/// | `size`    | 1–2    | Number of remaining bytes in the packet (starting at `ecmd` through `crc16`), encoded as a [`VarU16`]. |
/// | `ecmd`    | 1      | A [CDC2 extended command opcode](ecmds). |
/// | `ack`     | 1      | A [CDC2 packet acknowledgement code](Cdc2Ack). |
/// | `payload` | n      | Encoded payload; potentially optional if a [NACK](Cdc2Ack) occurs |
/// | `crc16`   | 2      | CRC16 checksum of the whole packet, computed with the [`VEX_CRC16`] algorithm. |
pub trait Cdc2Reply: CdcReply {
    const ECMD: u8;
}

pub(crate) fn decode_cdc2_reply<R: Cdc2Reply, P: Decode>(
    data: &mut &[u8],
) -> Result<Result<P, Cdc2Ack>, DecodeError> {
    let original_data = *data;

    if <[u8; 2]>::decode(data)? != R::HEADER {
        return Err(DecodeError::new::<R>(DecodeErrorKind::InvalidHeader));
    }

    let cmd = u8::decode(data)?;
    if cmd != R::CMD {
        return Err(DecodeError::new::<R>(DecodeErrorKind::UnexpectedByte {
            name: "cmd",
            value: cmd,
            expected: &[R::CMD],
        }));
    }

    let payload_size = VarU16::decode(data)?;
    let payload_size_size = payload_size.size() as usize;
    let payload_size = payload_size.into_inner();

    // Calculate the crc16 from the start of the packet up to the crc bytes.
    //
    // `payload_size` gives us the size of the payload including the CRC, and ecmd,
    // so the checksum range is effectively:
    // sizeof(header) + sizeof(cmd) + sizeof(payload_size) + payload_size - 2
    // which is 2 + 1 + (either 2 or 1) + payload_size - 2
    let expected_crc16 =
        decode_crc16::<R>(original_data.get(0..((payload_size as usize) + payload_size_size + 1)))?;

    let ecmd = u8::decode(data)?;
    if ecmd != R::ECMD {
        return Err(DecodeError::new::<R>(DecodeErrorKind::UnexpectedByte {
            name: "ecmd",
            value: ecmd,
            expected: &[R::ECMD],
        }));
    }

    let ack = Cdc2Ack::decode(data)?;
    let mut payload_data = data
        .get(0..(payload_size as usize) - 4)
        .ok_or_else(|| DecodeError::new::<R>(DecodeErrorKind::UnexpectedEnd))?;

    *data = &data[(payload_size as usize) - 4..];
    let payload = if ack == Cdc2Ack::Ack {
        Ok(P::decode(&mut payload_data)?)
    } else {
        Err(ack)
    };

    let crc16 = u16::decode(data)?;
    if crc16 != expected_crc16 {
        return Err(DecodeError::new::<R>(DecodeErrorKind::Checksum {
            value: crc16,
            expected: expected_crc16,
        }));
    }

    Ok(payload)
}

/// CDC2 packet opcodes.
pub mod ecmds {
    // internal filesystem operations
    pub const FILE_CTRL: u8 = 0x10;
    pub const FILE_INIT: u8 = 0x11;
    pub const FILE_EXIT: u8 = 0x12;
    pub const FILE_WRITE: u8 = 0x13;
    pub const FILE_READ: u8 = 0x14;
    pub const FILE_LINK: u8 = 0x15;
    pub const FILE_DIR: u8 = 0x16;
    pub const FILE_DIR_ENTRY: u8 = 0x17;
    pub const FILE_LOAD: u8 = 0x18;
    pub const FILE_GET_INFO: u8 = 0x19;
    pub const FILE_SET_INFO: u8 = 0x1A;
    pub const FILE_ERASE: u8 = 0x1B;
    pub const FILE_USER_STAT: u8 = 0x1C;
    pub const FILE_VISUALIZE: u8 = 0x1D;
    pub const FILE_CLEANUP: u8 = 0x1E;
    pub const FILE_FORMAT: u8 = 0x1F;

    // system
    pub const SYS_FLAGS: u8 = 0x20;
    pub const DEV_STATUS: u8 = 0x21;
    pub const SYS_STATUS: u8 = 0x22;
    pub const FDT_STATUS: u8 = 0x23;
    pub const LOG_STATUS: u8 = 0x24;
    pub const LOG_READ: u8 = 0x25;
    pub const RADIO_STATUS: u8 = 0x26;
    pub const USER_READ: u8 = 0x27;
    pub const SYS_SCREEN_CAP: u8 = 0x28;
    pub const SYS_USER_PROG: u8 = 0x29;
    pub const SYS_DASH_TOUCH: u8 = 0x2A;
    pub const SYS_DASH_SEL: u8 = 0x2B;
    pub const SYS_DASH_EBL: u8 = 0x2C;
    pub const SYS_DASH_DIS: u8 = 0x2D;
    pub const SYS_KV_LOAD: u8 = 0x2E;
    pub const SYS_KV_SAVE: u8 = 0x2F;

    // catalog
    pub const SYS_C_INFO_14: u8 = 0x31;
    pub const SYS_C_INFO_58: u8 = 0x32;

    // controller - only works over wired a controller connection
    pub const CON_RADIO_INIT_MODE: u8 = 0x20; //reinitializes radio as VN or BTll
    pub const CON_RADIO_CONFIGURE: u8 = 0x24;
    pub const CON_RADIO_INFO: u8 = 0x35;
    pub const CON_VER_FLASH: u8 = 0x39;
    pub const CON_GET_STATUS_PKT: u8 = 0x3A; //returns the same raw data as is sent to brain
    pub const CON_FLASH_ERASE: u8 = 0x3B;
    pub const CON_FLASH_WRITE: u8 = 0x3C;
    pub const CON_FLASH_VALIDATE: u8 = 0x3E;
    pub const CON_RADIO_FORCE: u8 = 0x3F;
    pub const CON_RADIO_MODE: u8 = 0x41; //changes channel type
    pub const CON_RADIO_CONTYPE: u8 = 0x42; //changes between Vexnet & Bluetooth
    pub const CON_RADIO_BACKLIGHT: u8 = 0x44;
    pub const CON_BRAINNAME: u8 = 0x45; //max sz 0x10
    pub const CON_TEAMNUMBER: u8 = 0x46; //max sz 8
    pub const CON_VER_EXPECT: u8 = 0x49;
    pub const CON_SET_SLOT: u8 = 0x72;
    pub const CON_COMP_CTRL: u8 = 0xC1;
    pub const CON_COMP_GET_SMARTFIELD: u8 = 0xC2; //returns the same raw data as is sent to smart field control

    // be careful!!
    pub const FACTORY_STATUS: u8 = 0xF1;
    pub const FACTORY_RESET: u8 = 0xF2;
    pub const FACTORY_PING: u8 = 0xF4;
    pub const FACTORY_PONG: u8 = 0xF5;
    pub const FACTORY_HW_STATUS: u8 = 0xF9;
    pub const FACTORY_CHAL: u8 = 0xFC;
    pub const FACTORY_RESP: u8 = 0xFD;
    pub const FACTORY_SPECIAL: u8 = 0xFE;
    pub const FACTORY_EBL: u8 = 0xFF;
}

/// CDC2 Packet Acknowledgement Codes
///
/// Encoded as part of a [`Cdc2ReplyPacket`], this type sends either an acknowledgement ([`Cdc2Ack::Ack`])
/// indicating that the command was successful or negative-acknowledgement ("NACK") if the command
/// failed in some way.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Error)]
#[repr(u8)]
pub enum Cdc2Ack {
    /// Acknowledges that a command has been processed successfully.
    #[error("Packet was recieved successfully. (NACK 0x76)")]
    Ack = 0x76,

    /// A general negative-acknowledgement (NACK) that is sometimes received.
    #[error("V5 device sent back a general negative-acknowledgement. (NACK 0xFF)")]
    Nack = 0xFF,

    // This is a sort-of hack. We can safely treat this byte as an ACK but it's the opcode
    // of a smartport packet that's been wrapped in very minimal CDC framing.
    #[error("Controller Smartfield status recieved succesfully. (Packet Opcode 0xA7)")]
    ControllerCompStatus = 0xA7,

    /// Returned by the brain when a CDC2 packet's CRC Checksum does not validate.
    #[error("Packet CRC checksum did not validate. (NACK 0xCE)")]
    NackPacketCrc = 0xCE,

    /// Returned by the brain when a packet's payload is of unexpected length (too short or too long).
    #[error("Packet payload length was either too short or too long. (NACK 0xD0)")]
    NackPacketLength = 0xD0,

    /// Returned by the brain when we attempt to transfer too much data.
    #[error("Attempted to transfer too much data. (NACK 0xD1)")]
    NackTransferSize = 0xD1,

    /// Returned by the brain when a program's CRC checksum fails.
    #[error("Program CRC checksum did not validate. (NACK 0xD2)")]
    NackProgramCrc = 0xD2,

    /// Returned by the brain when there is an error with the program file.
    #[error("Invalid program file. (NACK 0xD3)")]
    NackProgramFile = 0xD3,

    /// Returned by the brain when we fail to initialize a file transfer before beginning file operations.
    #[error(
        "Attempted to perform a file transfer operation before one was initialized. (NACK 0xD4)"
    )]
    NackUninitializedTransfer = 0xD4,

    /// Returned by the brain when we initialize a file transfer incorrectly.
    #[error("File transfer was initialized incorrectly. (NACK 0xD5)")]
    NackInvalidInitialization = 0xD5,

    /// Returned by the brain when we fail to pad a transfer to a four byte boundary.
    #[error("File transfer was not padded to a four byte boundary. (NACK 0xD6)")]
    NackAlignment = 0xD6,

    /// Returned by the brain when the addr on a file transfer does not match
    #[error("File transfer address did not match. (NACK 0xD7)")]
    NackAddress = 0xD7,

    /// Returned by the brain when the download length on a file transfer does not match
    #[error("File transfer download length did not match. (NACK 0xD8)")]
    NackIncomplete = 0xD8,

    /// Returned by the brain when a file transfer attempts to access a directory that does not exist
    #[error("Attempted to transfer file to a directory that does not exist. (NACK 0xD9)")]
    NackNoDirectory = 0xD9,

    /// Returned when the limit for user files has been reached
    #[error("Limit for user files has been reached. (NACK 0xDA)")]
    NackMaxUserFiles = 0xDA,

    /// Returned when a file already exists and we did not specify overwrite when initializing the transfer
    #[error("File already exists. (NACK 0xDB)")]
    NackFileAlreadyExists = 0xDB,

    /// Returned when the filesystem is full.
    #[error("Filesystem storage is full. (NACK 0xDC)")]
    NackFileStorageFull = 0xDC,

    /// Packet timed out.
    #[error("Packet timed out. (NACK 0x00)")]
    Timeout = 0x00,

    /// Internal Write Error.
    #[error("Internal write error occurred. (NACK 0x01)")]
    WriteError = 0x01,
}

impl Decode for Cdc2Ack {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        match u8::decode(data)? {
            0x76 => Ok(Self::Ack),
            0xFF => Ok(Self::Nack),
            0xCE => Ok(Self::NackPacketCrc),
            0xD0 => Ok(Self::NackPacketLength),
            0xD1 => Ok(Self::NackTransferSize),
            0xD2 => Ok(Self::NackProgramCrc),
            0xD3 => Ok(Self::NackProgramFile),
            0xD4 => Ok(Self::NackUninitializedTransfer),
            0xD5 => Ok(Self::NackInvalidInitialization),
            0xD6 => Ok(Self::NackAlignment),
            0xD7 => Ok(Self::NackAddress),
            0xD8 => Ok(Self::NackIncomplete),
            0xD9 => Ok(Self::NackNoDirectory),
            0xDA => Ok(Self::NackMaxUserFiles),
            0xDB => Ok(Self::NackFileAlreadyExists),
            0xDC => Ok(Self::NackFileStorageFull),
            0xA7 => Ok(Self::ControllerCompStatus),
            0x00 => Ok(Self::Timeout),
            0x01 => Ok(Self::WriteError),
            v => Err(DecodeError::new::<Self>(DecodeErrorKind::UnexpectedByte {
                name: "Cdc2Ack",
                value: v,
                expected: &[
                    0x76, 0xFF, 0xCE, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9,
                    0xDA, 0xDB, 0xDC, 0x00, 0x01,
                ],
            })),
        }
    }
}
