#[macro_use]
mod macros;
mod metadata;
mod stream_name;

pub use metadata::*;
pub use stream_name::*;
pub use thalo_derive::*;

use std::{fmt, time};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Represents an aggregate root in an event-sourced system.
///
/// An aggregate root is the entry-point for the cluster of entities and value objects
/// that are changed together in response to commands. This trait defines the behavior
/// for an aggregate root, including initializing, handling commands, and applying events.
///
/// Types implementing this trait must define an identifier, command type, event type,
/// and error type.
pub trait Aggregate {
    /// The identifier type for the aggregate.
    ///
    /// This is usually a unique identifier that can be created from a `String`.
    type ID: From<String>;

    /// The type of commands that this aggregate can handle.
    ///
    /// Commands are inputs to the aggregate that cause state changes,
    /// typically after being validated and generating domain events.
    type Command: DeserializeOwned;

    /// The type of events that this aggregate can produce.
    ///
    /// Events are the result of handling commands, representing state changes
    /// that have occurred to the aggregate.
    type Events: Events;

    /// The type of error that can occur when handling a command.
    ///
    /// This error is used to represent any issue encountered during command handling,
    /// such as validation errors or domain rule violations.
    type Error: fmt::Display;

    /// Initializes an aggregate with the given identifier.
    ///
    /// This method is called to create a new instance of an aggregate root.
    ///
    /// # Arguments
    /// * `id` - The identifier for the aggregate.
    ///
    /// # Returns
    /// Returns a new instance of the implementing aggregate type.
    fn init(id: Self::ID) -> Self;

    /// Applies an event to the aggregate.
    ///
    /// This method is used to mutate the state of the aggregate based on the given event.
    ///
    /// # Arguments
    /// * `evt` - The event to apply to the aggregate.
    fn apply(&mut self, evt: <Self::Events as Events>::Event);

    /// Handles a command and produces events.
    ///
    /// This method is called when a command is dispatched to the aggregate.
    /// If successful, it returns a list of events that represent the state changes
    /// resulting from the command.
    ///
    /// # Arguments
    /// * `cmd` - The command to handle.
    ///
    /// # Returns
    /// On success, returns `Ok` with a vector of resultant events.
    /// On failure, returns `Err` with an appropriate error.
    fn handle(
        &self,
        cmd: Self::Command,
    ) -> Result<Vec<<Self::Events as Events>::Event>, Self::Error>;
}

pub trait Events {
    type Event: Serialize + DeserializeOwned;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context<'a> {
    pub id: u64,
    pub stream_name: StreamName<'a>,
    pub position: u64,
    pub metadata: Metadata<'a>,
    pub time: time::SystemTime,
}

impl Default for Context<'_> {
    fn default() -> Self {
        Context {
            id: 0,
            stream_name: StreamName::default(),
            position: 0,
            metadata: Metadata::default(),
            time: time::SystemTime::now(),
        }
    }
}

#[doc(hidden)]
pub mod __macro_helpers {
    pub use serde_json;
    pub use wit_bindgen;
}
