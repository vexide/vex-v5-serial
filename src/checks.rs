/// Packet decoding checks
use bitflags::bitflags;

bitflags! {
    /// These flags determine what checks recieve_extended will perform
    /// on the recieved packet.
    pub struct VexExtPacketChecks: u8 {
        /// Bit 1 requires that we check the ACK value
        const ACK = 0b00000001;
        /// Bit 2 requires that we check the CRC
        const CRC = 0b00000010;
        /// Bit 3 requires that we check the Length of the packet
        const LENGTH = 0b00000100;
    }
}
