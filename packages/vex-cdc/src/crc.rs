use crc::Crc;

use crate::{DecodeError, DecodeErrorKind};

/// [CRC16 error-detecting algorithm](https://en.wikipedia.org/wiki/Cyclic_redundancy_check)
/// used in CDC2 packets.
pub const VEX_CRC16: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_XMODEM);

/// [CRC32 error-detecting algorithm](https://en.wikipedia.org/wiki/Cyclic_redundancy_check)
/// used for file uploads.
pub const VEX_CRC32: Crc<u32> = Crc::<u32>::new(&crc::Algorithm {
    poly: 0x04C11DB7,
    init: 0x00000000,
    refin: false,
    refout: false,
    xorout: 0x00000000,
    check: 0x89A1897F,
    residue: 0x00000000,
    width: 32,
});

#[inline]
pub(crate) fn crc16<T>(buf: Option<&[u8]>) -> Result<u16, DecodeError> {
    Ok(VEX_CRC16
        .checksum(buf.ok_or_else(|| DecodeError::new::<T>(DecodeErrorKind::UnexpectedEnd))?)
        .to_be())
}
