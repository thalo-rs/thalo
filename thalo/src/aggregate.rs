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

/// Unique type identifier.
pub trait TypeId {
    /// Returns a unique identifier for the given type.
    fn type_id() -> &'static str;
}

/// Generic aggregate error.
pub struct AggregateError {
    code: Option<String>,
    msg: Option<String>,
}

impl AggregateError {
    /// Create a new error with a code and message.
    pub fn new(code: impl Into<String>, msg: impl Into<String>) -> Self {
        AggregateError {
            code: Some(code.into()),
            msg: Some(msg.into()),
        }
    }

    /// Create a new error with a code only.
    pub fn new_code(code: impl Into<String>) -> Self {
        AggregateError {
            code: Some(code.into()),
            msg: None,
        }
    }

    /// Create a new error with a message only.
    pub fn new_message(msg: impl Into<String>) -> Self {
        AggregateError {
            code: None,
            msg: Some(msg.into()),
        }
    }

    /// Get the error code.
    pub fn code(&self) -> Option<&String> {
        self.code.as_ref()
    }

    /// Get the error message.
    pub fn msg(&self) -> Option<&String> {
        self.msg.as_ref()
    }
}
