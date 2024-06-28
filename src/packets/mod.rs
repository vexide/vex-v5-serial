use crate::decode::{Decode, DecodeError};

pub mod capture;
pub mod cdc;
pub mod cdc2;
pub mod controller;
pub mod dash;
pub mod device;
pub mod factory;
pub mod file;
pub mod kv;
pub mod log;
pub mod radio;
pub mod program;
pub mod system;

/// Header byte sequence used for all device-bound packets.
pub(crate) const DEVICE_BOUND_HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

/// Header byte sequence used for all host-bound packets.
pub(crate) const HOST_BOUND_HEADER: [u8; 2] = [0xAA, 0x55];

/// Decodes a [`HostBoundPacket`]'s header sequence.
pub(crate) fn decode_header(data: impl IntoIterator<Item = u8>) -> Result<[u8; 2], DecodeError> {
    let mut data = data.into_iter();
    let header = Decode::decode(&mut data)?;
    if header != HOST_BOUND_HEADER {
        return Err(DecodeError::InvalidHeader);
    }
    Ok(header)
}