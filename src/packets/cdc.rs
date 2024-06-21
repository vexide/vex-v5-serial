use super::{DeviceBoundCdcPacket, HostBoundPacket};

/// CDC (Simple) Command Packet
///
/// Encodes a simple device-bound message over the protocol containing
/// an ID and a payload.
pub type CdcCommandPacket<const ID: u8, P> = DeviceBoundCdcPacket<ID, P>;

/// CDC (Simple) Command Reply Packet
///
/// Encodes a reply payload to a [`CdcCommand`] packet for a given ID.
pub type CdcReplyPacket<const ID: u8, P> = HostBoundPacket<P, ID>;
