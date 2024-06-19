use crate::v5::J2000_EPOCH;

pub mod capture;
pub mod cdc;
pub mod cdc2;
pub mod dash;
pub mod file;
pub mod kv;
pub mod log;
pub mod slot;
pub mod status;
pub mod system;

/// Encodes a u16 as an unsigned var short.
///
/// # Panics
///
/// This function panics if the input value is too large to fit within 15 bits
pub(crate) fn encode_var_u16(val: u16) -> Vec<u8> {
    if val > (u16::MAX >> 1) {
        panic!("Input value too large to fit in unsigned var short");
    }

    if val > (u8::MAX >> 1) as _ {
        let mut val = val.to_le_bytes();
        val[0] |= 1 << 7;
        val.to_vec()
    } else {
        let val = val as u8;
        vec![val]
    }
}

pub(crate) fn j2000_timestamp() -> u32 {
    (chrono::Utc::now().timestamp() - J2000_EPOCH as i64) as u32
}

/// Attempts to code a string as a fixed length string.
///
/// # Note
///
/// This does not add a null terminator!
pub(crate) fn encode_unterminated_fixed_string<const LEN: usize>(
    string: String,
) -> Option<[u8; LEN]> {
    let mut encoded = [0u8; LEN];

    let string_bytes = string.into_bytes();
    if string_bytes.len() > encoded.len() {
        return None;
    }

    encoded[..string_bytes.len()].copy_from_slice(&string_bytes);

    Some(encoded)
}

/// Attempts to encode a string as a fixed length string.
///
/// # Note
///
///The output of this function will always be ``LEN + 1`` bytes on success.
pub(crate) fn encode_terminated_fixed_string<const LEN: usize>(string: String) -> Option<Vec<u8>> {
    let unterminated = encode_unterminated_fixed_string(string);

    unterminated.map(|bytes: [u8; LEN]| {
        let mut bytes = Vec::from(bytes);
        bytes.push(0);
        bytes
    })
}

/// A trait that allows for encoding a structure into a byte sequence.
pub trait Encode {
    /// Encodes a structure into a byte sequence.
    fn encode(&self) -> Vec<u8>;
    fn into_encoded(self) -> Vec<u8>
    where
        Self: Sized,
    {
        self.encode()
    }
}

/// Device-bound Communications Packet
///
/// This structure encodes a data payload and ID that is intended to be sent from
/// a host machine to a V5 device over the serial protocol. This is typically done
/// through either a [`CdcCommandPacket`] or a [`Cdc2CommandPacket`].
pub struct DeviceBoundPacket<P: Encode, const ID: u8> {
    /// Device-bound Packet Header
    ///
    /// This must be `Self::HEADER` or `[0xC9, 0x36, 0xB8, 0x47]`.
    header: [u8; 4],

    /// Packet Payload
    ///
    /// Contains data for a given packet that be encoded and sent over serial to the device.
    payload: P,
}
impl<P: Encode, const ID: u8> Encode for DeviceBoundPacket<P, ID> {
    fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&self.header);
        encoded.push(ID);

        let size = self.payload.encode().len() as u16;
        encoded.extend(encode_var_u16(size));

        encoded.extend_from_slice(&self.payload.encode());
        encoded
    }
}

impl<P: Encode, const ID: u8> DeviceBoundPacket<P, ID> {
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
/// through either a [`CdcReplyPacket`] or a [`Cdc2ReplyPacket`].
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
impl Encode for Version {
    fn encode(&self) -> Vec<u8> {
        vec![self.major, self.minor, self.build, self.beta]
    }
}