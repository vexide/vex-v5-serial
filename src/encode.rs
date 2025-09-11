pub struct MessageEncoder<'a> {
    data: &'a mut [u8],
    pos: usize,
}

impl<'a> MessageEncoder<'a> {
    pub const fn new(data: &'a mut [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn write<T: Encode>(&mut self, value: &T) {
        let data = &mut self.data[self.pos..];

        value.encode(data);
        self.pos += value.size();
    }

    #[inline]
    pub const fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    #[inline]
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    #[must_use]
    pub const fn get_ref(&self) -> &[u8] {
        self.data
    }
}

pub trait Encode {
    /// Returns the number of bytes this value will take when encoded.
    fn size(&self) -> usize;

    /// Encodes this instance into the provided byte slice.
    fn encode(&self, data: &mut [u8]);
}

// pub trait Decode {
//     fn decode(&self, data: &[u8]) -> Result<usize, EncodeError>;
// }

macro_rules! impl_encode_for_primitive {
    ($($t:ty),*) => {
        $(
            impl Encode for $t {
                fn size(&self) -> usize {
                    size_of::<Self>()
                }

                fn encode(&self, data: &mut [u8]) {
                    let size = self.size();
                    data[..size].copy_from_slice(&self.to_le_bytes());
                }
            }
        )*
    };
}

impl_encode_for_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

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

impl Encode for Vec<u8> {
    fn size(&self) -> usize {
        self.len()
    }

    fn encode(&self, data: &mut [u8]) {
        self.as_slice().encode(data)
    }
}