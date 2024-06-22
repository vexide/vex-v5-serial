use std::future::Future;

use crate::connection::{device::Device, DeviceError};

pub mod file;

pub trait Command {
    type Output;

    fn execute(
        &mut self,
        device: &mut Device,
    ) -> impl Future<Output = Result<Self::Output, DeviceError>>;
}
