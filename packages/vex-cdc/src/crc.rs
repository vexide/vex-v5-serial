use crc::Crc;

/// [CRC16 error-detecting algorithm](https://en.wikipedia.org/wiki/Cyclic_redundancy_check)
/// used in CDC packets.
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
