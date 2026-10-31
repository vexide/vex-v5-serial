use std::time::{Duration, Instant};

use vex_cdc::{Decode, DecodeError, DecodeErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    pub bytes: Vec<u8>,
    pub used: bool,
    pub timestamp: Instant,
}

impl Frame {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            used: false,
            timestamp: Instant::now(),
        }
    }

    pub fn is_obsolete(&self, timeout: Duration) -> bool {
        self.timestamp.elapsed() > timeout || self.used
    }

    /// Decodes the packet into the given type.
    ///
    /// If successful, marks the packet as used.
    ///
    /// # Note
    ///
    /// This function will **NOT** fail if the packet has already been used.
    pub fn decode_and_use<D: Decode>(&mut self) -> Option<Result<D, DecodeError>> {
        match D::decode(&mut self.bytes.as_slice()) {
            Ok(pkt) => {
                self.used = true;
                Some(Ok(pkt))
            }
            Err(err) => match err.kind() {
                DecodeErrorKind::UnexpectedByte { name, .. } if name == "cmd" || name == "ecmd" => {
                    None
                }
                _ => Some(Err(err)),
            },
        }
    }
}

/// Removes old and used packets from the incoming packets buffer.
pub(crate) fn trim_packets(packets: &mut Vec<Frame>) {
    log::trace!("Trimming packets. Length before: {}", packets.len());

    // Remove packets that are obsolete
    packets.retain(|packet| !packet.is_obsolete(Duration::from_secs(2)));

    log::trace!("Trimmed packets. Length after: {}", packets.len());
}
