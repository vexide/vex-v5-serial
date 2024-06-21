use crc::Crc;

/// Vex uses CRC16/XMODEM as the CRC16.
pub const VEX_CRC16: crc::Crc<u16> = Crc::<u16>::new(&crc::CRC_16_XMODEM);

/// Vex uses a CRC32 that I found on page 6 of this document:
/// <https://www.matec-conferences.org/articles/matecconf/pdf/2016/11/matecconf_tomsk2016_04001.pdf>
/// I literally just discovered it by guessing and checking against the PROS implementation.
pub const VEX_CRC32: crc::Crc<u32> = Crc::<u32>::new(&crc::Algorithm {
    poly: 0x04C11DB7,
    init: 0x00000000,
    refin: false,
    refout: false,
    xorout: 0x00000000,
    check: 0x89A1897F,
    residue: 0x00000000,
    width: 32,
});
