/// A type that can be encoded into a sequence of bytes.
pub trait Encode {
    /// Returns the number of bytes this value will take when encoded.
    fn size(&self) -> usize;

    /// Encodes this instance into the provided byte slice.
    fn encode(&self, data: &mut [u8]);
}

macro_rules! impl_encode_for_primitive {
    ($($t:ty),*) => {
        $(
            impl Encode for $t {
                fn size(&self) -> usize {
                    size_of::<Self>()
                }

                fn encode(&self, data: &mut [u8]) {
                    data[..size_of::<Self>()].copy_from_slice(&self.to_le_bytes());
                }
            }
        )*
    };
}

impl_encode_for_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl Encode for () {
    fn size(&self) -> usize {
        0
    }
    fn encode(&self, _data: &mut [u8]) {}
}

impl Encode for &[u8] {
    fn size(&self) -> usize {
        self.len()
    }

    fn encode(&self, data: &mut [u8]) {
        data[..self.len()].copy_from_slice(self);
    }
}

impl<const N: usize> Encode for [u8; N] {
    fn size(&self) -> usize {
        N
    }

    fn encode(&self, data: &mut [u8]) {
        data[..N].copy_from_slice(self);
    }
}

impl Encode for alloc::vec::Vec<u8> {
    fn size(&self) -> usize {
        self.len()
    }

    fn encode(&self, data: &mut [u8]) {
        self.as_slice().encode(data)
    }
}
