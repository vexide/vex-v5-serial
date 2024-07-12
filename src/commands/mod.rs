use std::future::Future;

use crate::connection::{Connection, ConnectionError};

pub mod file;
#[cfg(feature = "screen-command")]
pub mod screen;

pub trait Command {
    type Output;

    fn execute<C: Connection + ?Sized>(
        &mut self,
        connection: &mut C,
    ) -> impl Future<Output = Result<Self::Output, ConnectionError>>;
}
