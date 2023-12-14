//! [Thalo] is an event sourcing runtime that leverages the power of WebAssembly
//! (wasm) through [wasmtime], combined with [sled] as an embedded event store.
//! It is designed to handle commands using compiled aggregate wasm components
//! and to persist the resulting events, efficiently managing the rebuilding of
//! aggregate states from previous events.
//!
//! [thalo]: https://github.com/thalo-rs/thalo
//! [wasmtime]: https://wasmtime.dev/
//! [sled]: https://sled.rs/
//!
//! # Counter Aggregate Example
//!
//! This example shows a basic counter aggregate, allowing the count to be
//! incremented by an amount.
//!
//! ```
//! # use std::convert::Infallible;
//! #
//! use serde::{Deserialize, Serialize};
//! use thalo::{events, export_aggregate, Aggregate, Apply, Command, Event, Handle};
//!
//! export_aggregate!(Counter);
//!
//! pub struct Counter {
//!     count: u64,
//! }
//!
//! impl Aggregate for Counter {
//!     type Command = CounterCommand;
//!     type Event = CounterEvent;
//!
//!     fn init(_id: String) -> Self {
//!         Counter { count: 0 }
//!     }
//! }
//!
//! #[derive(Command, Deserialize)]
//! pub enum CounterCommand {
//!     Increment { amount: u64 },
//! }
//!
//! impl Handle<CounterCommand> for Counter {
//!     type Error = Infallible;
//!
//!     fn handle(&self, cmd: CounterCommand) -> Result<Vec<CounterEvent>, Self::Error> {
//!         match cmd {
//!             CounterCommand::Increment { amount } => events![Incremented { amount }],
//!         }
//!     }
//! }
//!
//! #[derive(Event, Serialize, Deserialize)]
//! pub enum CounterEvent {
//!     Incremented(Incremented),
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! pub struct Incremented {
//!     pub amount: u64,
//! }
//!
//! impl Apply<Incremented> for Counter {
//!     fn apply(&mut self, event: Incremented) {
//!         self.count += event.amount;
//!     }
//! }
//! ```

#[macro_use]
mod macros;
pub mod event_store;
pub mod stream_name;

pub use thalo_derive::*;
/// Re-exports of [tracing](::tracing) macros.
pub mod tracing {
    pub use tracing::{
        debug, debug_span, enabled, error, error_span, info, info_span, trace, trace_span, warn,
        warn_span,
    };
}

/// Represents an aggregate root in an event-sourced system.
///
/// An aggregate root is the entry-point for the cluster of entities and that
/// are changed together in response to commands.
///
/// *See the [crate level docs](crate#counter-aggregate-example) for an example
/// aggregate.*
pub trait Aggregate {
    /// The type of commands that this aggregate can handle.
    ///
    /// Commands are inputs to the aggregate that cause state changes,
    /// typically after being validated and generating domain events.
    /// The type used here typically derives the [Command] derive macro.
    type Command;

    /// The type of events that this aggregate can produce.
    ///
    /// Events are the result of handling commands, representing state changes
    /// that have occurred to the aggregate.
    /// The type used here typically derives the [Event] derive macro.
    type Event;

    /// Initializes an aggregate with the given identifier.
    ///
    /// This method is called to create a new instance of an aggregate root
    /// with a default state.
    fn init(id: String) -> Self;
}

/// Handles a command, returning events.
///
/// Commands use the aggregates state to validate business rules, and returns
/// events which are later used to update the aggregate state.
///
/// # Example
///
/// ```
/// use thalo::{events, Handle};
///
/// pub struct Increment {
///     pub amount: u64,
/// }
///
/// impl Handle<Increment> for Counter {
///     type Error = &'static str;
///
///     fn handle(&self, cmd: Increment) -> Result<Vec<CounterEvent>, Self::Error> {
///         if self.count + cmd.amount > 100_000 {
///             return Err("count would be too high");
///         }
///         
///         events![ Incremented { amount: cmd.amount } ]
///     }
/// }
/// ```
pub trait Handle<C>: Aggregate {
    type Error;

    fn handle(&self, cmd: C) -> Result<Vec<Self::Event>, Self::Error>;
}

/// Applies an event, updating the aggregate state.
///
/// Events modify aggregate state, and are emitted as the result of commands.
///
/// # Example
///
/// ```
/// use thalo::Apply;
///
/// pub struct Increment {
///     pub amount: u64,
/// }
///
/// impl Apply<Incremented> for Counter {
///     fn apply(&mut self, event: Incremented) {
///         self.count += self.amount;
///     }
/// }
/// ```
pub trait Apply<E>: Aggregate {
    fn apply(&mut self, event: E);
}

#[doc(hidden)]
pub struct State<T>(pub T);

impl<T> Aggregate for State<T>
where
    T: Aggregate,
{
    type Command = T::Command;
    type Event = T::Event;

    fn init(id: String) -> Self {
        State(T::init(id))
    }
}

#[doc(hidden)]
pub mod __macro_helpers {
    use serde_json::Value;
    pub use {serde_json, tracing, tracing_tunnel, wit_bindgen};

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
