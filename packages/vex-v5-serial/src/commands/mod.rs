use std::future::Future;

use crate::Connection;

#[cfg(feature = "file-commands")]
pub mod file;
#[cfg(feature = "screen-commands")]
pub mod screen;
