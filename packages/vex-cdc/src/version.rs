use crate::decode::{Decode, DecodeError};
use crate::encode::Encode;

/// A VEXos firmware version.
///
/// This type represents a version identifier for VEXos firmware. VEXos is
/// versioned using a slightly modified [semantic versioning] scheme.
///
/// [semantic versioning]: https://semver.org/
///
/// This type implements `PartialOrd`, meaning it can be compared to other
/// instances of itself.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Version {
    /// The major version
    pub major: u8,
    /// The minor version
    pub minor: u8,
    /// The build version
    pub build: u8,
    /// The beta version
    pub beta: u8,
}

impl Encode for Version {
    fn size(&self) -> usize {
        4
    }

    fn encode(&self, data: &mut [u8]) {
        data[0] = self.major;
        data[1] = self.minor;
        data[2] = self.build;
        data[3] = self.beta;
    }
}

impl Decode for Version {
    fn decode(data: &mut &[u8]) -> Result<Self, DecodeError> {
        let major = u8::decode(data)?;
        let minor = u8::decode(data)?;
        let build = u8::decode(data)?;
        let beta = u8::decode(data)?;

        Ok(Self {
            major,
            minor,
            build,
            beta,
        })
    }
}
