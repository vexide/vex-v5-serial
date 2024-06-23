use std::future::Future;

use crate::connection::{serial::SerialConnection, ConnectionError};

pub mod file;
pub mod screen;

pub trait Command {
    type Output;

    fn execute(
        &mut self,
        connection: &mut SerialConnection,
    ) -> impl Future<Output = Result<Self::Output, ConnectionError>>;
}
