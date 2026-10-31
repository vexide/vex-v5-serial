#[cfg(feature = "async")]
mod asynchronous;
#[cfg(feature = "async")]
pub use asynchronous::*;

#[cfg(feature = "blocking")]
pub mod blocking;