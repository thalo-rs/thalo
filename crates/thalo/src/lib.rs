#![allow(unused_macros)]

mod aggregate;
mod command;
#[cfg(feature = "consumer")]
pub mod consumer;
mod error;
mod event;
mod macros;

pub use aggregate::{wit_aggregate, Aggregate};
pub use command::*;
pub use error::{Error, ErrorKind};
pub use event::*;
// Re-exported for thalo_macros
#[doc(hidden)]
pub use serde_json;
pub use thalo_macros::{Aggregate, Commands, Events};
