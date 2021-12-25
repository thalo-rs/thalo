//! Thalo

#![deny(missing_docs)]

#[cfg(feature = "actor")]
pub mod actor;

pub mod aggregate;

pub mod event;

#[cfg(feature = "event-store")]
pub mod event_store;

#[cfg(feature = "event-stream")]
pub mod event_stream;

#[doc(hidden)]
#[cfg(feature = "tests-cfg")]
pub mod tests_cfg;
