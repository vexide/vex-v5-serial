use crate::decode::{Decode, DecodeError};

pub struct Array<T> {
    data: Vec<T>,
}
impl<T> Array<T> {
    pub fn new(data: Vec<T>) -> Self {
        Self { data }
    }
    pub fn into_inner(self) -> Vec<T> {
        self.data
    }
}
impl<T: Decode> Array<T> {
    pub fn decode_with_len(
        data: impl IntoIterator<Item = u8>,
        len: usize,
    ) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::decode(&mut data)?);
        }
        Ok(Self { data: vec })
    }
    pub fn decode_with_max_len(
        data: impl IntoIterator<Item = u8>,
        max_len: usize,
    ) -> Result<Self, DecodeError> {
        let mut data = data.into_iter();
        let mut vec = Vec::with_capacity(max_len);
        for _ in 0..max_len {
            if let Ok(item) = T::decode(&mut data) {
                vec.push(item);
            } else {
                break;
            }
        }
        Ok(Self { data: vec })
    }
}
