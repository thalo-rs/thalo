//! Thalo is an event sourcing runtime that leverages the power of WebAssembly (wasm) through [wasmtime], combined with [sled] as an embedded event store.
//! It is designed to handle commands using compiled aggregate wasm components and to persist the resulting events, efficiently managing the rebuilding of aggregate states from previous events.
//!
//! [wasmtime]: https://wasmtime.dev/
//! [sled]: https://sled.rs/
//!
//! # Example
//!
//! #### Counter Aggregate Example
//!
//! This example shows a basic counter aggregate, allowing the count to be incremented and decremented.
//!
//! **Commands**
//!
//! - `Increment`
//! - `Decrement`
//!
//! **Events**
//!
//! - `Incremented`
//! - `Decremented`
//!
//! ```
//! use serde::{Deserialize, Serialize};
//! use thalo::{export_aggregate, Aggregate, Events};
//!
//! export_aggregate!(Counter);
//!
//! pub struct Counter {
//!     count: i64,
//! }
//!
//! impl Aggregate for Counter {
//!     type ID = String;
//!     type Command = CounterCommand;
//!     type Events = CounterEvents;
//!     type Error = &'static str;
//!
//!     fn init(_id: Self::ID) -> Self {
//!         Counter { count: 0 }
//!     }
//!
//!     fn apply(&mut self, evt: Event) {
//!         use Event::*;
//!
//!         match evt {
//!             Incremented(IncrementedV1 { amount }) => self.count += amount,
//!             Decremented(DecrementedV1 { amount }) => self.count -= amount,
//!         }
//!     }
//!
//!     fn handle(&self, cmd: Self::Command) -> Result<Vec<Event>, Self::Error> {
//!         use CounterCommand::*;
//!         use Event::*;
//!
//!         match cmd {
//!             Increment { amount } => Ok(vec![Incremented(IncrementedV1 { amount })]),
//!             Decrement { amount } => Ok(vec![Decremented(DecrementedV1 { amount })]),
//!         }
//!     }
//! }
//!
//! #[derive(Deserialize)]
//! pub enum CounterCommand {
//!     Increment { amount: i64 },
//!     Decrement { amount: i64 },
//! }
//!
//! #[derive(Events)]
//! pub enum CounterEvents {
//!     Incremented(IncrementedV1),
//!     Decremented(DecrementedV1),
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! pub struct IncrementedV1 {
//!     amount: i64,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! pub struct DecrementedV1 {
//!     amount: i64,
//! }
//! ```

#[macro_use]
mod macros;
mod stream_name;

pub use stream_name::*;
pub use thalo_derive::*;

use std::fmt;

use serde::{de::DeserializeOwned, Serialize};

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
    /// * `event` - The event to apply to the aggregate.
    fn apply(&mut self, event: <Self::Events as Events>::Event);

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

/// The `Events` trait, along with its associated `Events` derive macro,
/// plays a pivotal role in defining and handling versioned events in Thalo.
///
/// It enables the creation of enum types where each variant represents a distinct
/// event type with potentially multiple versions. This setup facilitates the evolution
/// of event structures over time, while maintaining backward compatibility.
///
/// # Key Concepts
///
/// - **Versioned Events:** Each enum variant in an `Events` implementation can contain
///   multiple versions of an event, ensuring a smooth transition between different event schemas.
///
/// - **Migration Path:** All event versions must implement `From<PreviousVersion>`, allowing
///   automatic conversion from older to newer versions.
///
/// - **Generated Event Enum:** A corresponding `Event` enum is automatically generated,
///   encapsulating the latest version of each event type.
///
/// - **Generated Event Names:** The event names for serialization are derived from the enum
///   variants and are suffixed with 'V1', 'V2', etc., to represent different versions.
///   The internal names of structs are not used in serialization.
///
/// # Example
///
/// ```rust
/// #[derive(thalo::Events)]
/// pub enum Events {
///     Incremented(IncrementedV1, IncrementedV2),
///     Decremented(DecrementedV1, DecrementedV2),
/// }
///
/// #[derive(Serialize, Deserialize)]
/// pub struct IncrementedV1 { /* fields */ }
///
/// #[derive(Serialize, Deserialize)]
/// pub struct IncrementedV2 { /* fields */ }
///
/// // Implement conversions between versions
/// impl From<IncrementedV1> for IncrementedV2 { /* conversion logic */ }
/// ```
///
/// In this example, `IncrementedV1` and `IncrementedV2` are versions of the `Incremented` event.
/// The macro generates an `Event` enum with the latest version (`IncrementedV2`), and during
/// serialization, these events will be tagged as `IncrementedV1` and `IncrementedV2`.
pub trait Events {
    /// The event type associated with this trait, typically generated by the `thalo::Events` macro.
    /// It should encompass all versions of an event.
    type Event: Serialize + DeserializeOwned;
}

#[doc(hidden)]
pub mod __macro_helpers {
    pub use serde_json;
    use serde_json::Value;
    pub use wit_bindgen;

    /// Extracts the event name and payload from an event json value.
    /// `{"EventName": {"foo": 1}}` returns `("EventName", {"foo": 1})`.
    pub fn extract_event_name_payload(value: Value) -> Result<(String, Value), &'static str> {
        let Value::Object(map) = value else {
            return Err("event is not an object");
        };

        let mut iter = map.into_iter();
        let Some((event, payload)) = iter.next() else {
            return Err("event is empty");
        };

        if iter.next().is_some() {
            return Err("event contains multiple keys");
        }

        Ok((event, payload))
    }
}
