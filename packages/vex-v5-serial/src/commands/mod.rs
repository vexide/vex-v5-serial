use std::future::Future;

use crate::Connection;

pub mod file;
#[cfg(feature = "screen-command")]
pub mod screen;

pub trait Command {
    type Output;

    fn execute<C: Connection + ?Sized>(
        self,
        connection: &mut C,
    ) -> impl Future<Output = Result<Self::Output, C::Error>>;
}
