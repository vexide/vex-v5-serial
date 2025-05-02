use crate::decode::{Decode, DecodeError, DecodeErrorKind};

/// A struct that allows for attempting to decode two different types and returning the successful one.
/// If neither are successful, an error is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice<L: Decode, R: Decode> {
    /// The left choice was decoded successfully and the right choice was not.
    Left(L),
    /// The right choice was decoded successfully and the left choice was not.
    Right(R),
    /// Both choices were decoded successfully.
    Either(L, R),
}
impl<L: Decode, R: Decode> Decode for Choice<L, R> {
    fn decode(data: impl IntoIterator<Item = u8>) -> Result<Self, crate::decode::DecodeError> {
        let data = data.into_iter().collect::<Vec<_>>();

        let left = L::decode(data.clone());
        let right = R::decode(data);

        match (left, right) {
            (Ok(l), Ok(r)) => Ok(Self::Either(l, r)),
            (Ok(l), Err(_)) => Ok(Self::Left(l)),
            (Err(_), Ok(r)) => Ok(Self::Right(r)),
            (Err(l), Err(r)) => Err(DecodeError::new::<Self>(DecodeErrorKind::BothChoicesFailed {
                left: Box::new(l),
                right: Box::new(r),
            })),
        }
    }
}
pub enum PrefferedChoice<L: Decode, R: Decode> {
    Left(L),
    Right(R),
}
impl<L: Decode, R: Decode> Choice<L, R> {
    /// Returns the left choice if it was decoded successfully, otherwise returns the right choice.
    pub fn prefer_left(self) -> PrefferedChoice<L, R> {
        match self {
            Self::Left(l) => PrefferedChoice::Left(l),
            Self::Either(l, _) => PrefferedChoice::Left(l),
            Self::Right(r) => PrefferedChoice::Right(r),
        }
    }
    /// Returns the right choice if it was decoded successfully, otherwise returns the left choice.
    #[allow(unused)]
    pub fn prefer_right(self) -> PrefferedChoice<L, R> {
        match self {
            Self::Right(r) => PrefferedChoice::Right(r),
            Self::Either(_, r) => PrefferedChoice::Right(r),
            Self::Left(l) => PrefferedChoice::Left(l),
        }
    }
}
