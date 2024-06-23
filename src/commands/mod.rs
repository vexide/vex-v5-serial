use std::future::Future;

use crate::connection::{Connection, ConnectionError};

pub mod file;
pub mod screen;

pub trait Command {
    type Output;

    fn execute<C: Connection>(
        &mut self,
        connection: &mut C,
    ) -> impl Future<Output = Result<Self::Output, ConnectionError>>;
}
