pub mod capture;
pub mod cdc;
pub mod cdc2;
pub mod dash;
pub mod status;
pub mod file;
pub mod kv;
pub mod log;
pub mod slot;
pub mod system;

/// Device-bound Communications Packet
/// 
/// This structure encodes a data payload and ID that is intended to be sent from
/// a host machine to a V5 device over the serial protocol. This is typically done
/// through either a [`CdcCommandReply`] or a [`Cdc2CommandReply`].
pub struct DeviceBoundPacket<P, const ID: u8> {
    /// Device-bound Packet Header
    /// 
    /// This must be `Self::HEADER` or `[0xC9, 0x36, 0xB8, 0x47]`.
    header: [u8; 4],

    /// Packet Payload
    /// 
    /// Contains data for a given packet that be encoded and sent over serial to the device.
    payload: P,
}

impl<P, const ID: u8> DeviceBoundPacket<P, ID> {
    /// Header byte sequence used for all device-bound packets.
    pub const HEADER: [u8; 4] = [0xC9, 0x36, 0xB8, 0x47];

    /// Creates a new device-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: Self::HEADER,
            payload,
        }
    }
}

/// Host-bound Communications Packet
/// 
/// This structure encodes a data payload and ID that is intended to be sent from
/// a V5 device to a host machine over the serial protocol. This is typically done
/// through either a [`CdcCommandReply`] or a [`Cdc2CommandReply`].
pub struct HostBoundPacket<P, const ID: u8> {
    /// Host-bound Packet Header
    /// 
    /// This must be `Self::HEADER` or `[0xAA, 0x55]`.
    header: [u8; 2],

    /// Packet Payload
    /// 
    /// Contains data for a given packet that be encoded and sent over serial to the host.
    payload: P,
}

impl<P, const ID: u8> HostBoundPacket<P, ID> {
    /// Header byte sequence used for all host-bound packets.
    pub const HEADER: [u8; 2] = [0xAA, 0x55];

    /// Creates a new host-bound packet with a given generic payload type.
    pub fn new(payload: P) -> Self {
        Self {
            header: Self::HEADER,
            payload,
        }
    }
}

pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub beta: u8,
}