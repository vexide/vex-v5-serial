use crate::decode::{Decode, DecodeError};
use crate::encode::{Encode, EncodeError};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub beta: u8,
}
impl Encode for Version {
    fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(vec![self.major, self.minor, self.build, self.beta])
    }
}
impl Decode for Version {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let major = u8::decode(&mut data)?;
        let minor = u8::decode(&mut data)?;
        let build = u8::decode(&mut data)?;
        let beta = u8::decode(&mut data)?;
        Ok(Self {
            major,
            minor,
            build,
            beta,
        })
    }
}
