//! Aggregates

use std::string;

use crate::event::EventType;

#[cfg(feature = "macros")]
pub use thalo_macros::{Aggregate, TypeId};

/// Consistency boundary around a domain entity responsible for handling commands and applying events.
pub trait Aggregate: TypeId {
    /// The ID type of the aggregate.
    type ID: string::ToString + Send + Sync;

    /// The event type resulted by a command.
    type Event: EventType + Send + Sync;

    /// Create a new instance from a given ID.
    ///
    /// The aggregate should be initialised with an initial state.
    fn new(id: Self::ID) -> Self;

    /// Returns a reference to the aggregate ID.
    fn id(&self) -> &Self::ID;

    /// Applies an event to update internal state.
    fn apply(&mut self, event: Self::Event);
}

/// Unique type identifier
pub trait TypeId {
    /// Returns a unique identifier for the given type
    fn type_id() -> &'static str;
}
