use crate::decode::{Decode, DecodeError};
use crate::encode::Encode;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
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
