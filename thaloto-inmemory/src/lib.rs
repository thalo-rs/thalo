//! An in memory implementation of [EventStore](thaloto::event_store::EventStore).
//!
//! This is useful for testing, but is not recommended
//! for production as the data does not persist to disk.
//!
//! Events are stored in a `Vec<EventRecord>`.

#![deny(missing_docs)]

pub use error::Error;
pub use event_store::InMemoryEventStore;

mod error;
mod event_store;
