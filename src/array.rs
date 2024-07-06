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
}

#[cfg(test)]
mod tests {
    use super::Array;

    #[test]
    fn decode() {
        let data: Vec<u8> = vec![0x01, 0x02, 0x03, 0x04];
        let mut data = data.into_iter();
        let len = data.len();
        let array = Array::<u8>::decode_with_len(&mut data, len).unwrap();

        // All the data should be consumed
        assert_eq!(data.len(), 0);

        assert_eq!(array.into_inner(), vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn decode_some() {
        let data: Vec<u8> = vec![0x01, 0x02, 0x03, 0x04];
        let mut data = data.into_iter();
        let array = Array::<u8>::decode_with_len(&mut data, 2).unwrap();

        // Only 2 bytes should be consumed
        assert_eq!(data.len(), 2);

        assert_eq!(array.into_inner(), vec![0x01, 0x02]);
        assert_eq!(data.collect::<Vec<_>>(), vec![0x03, 0x04]);
    }
}
