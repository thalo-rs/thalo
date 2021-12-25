//! Events

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::aggregate::Aggregate;

/// An event with additional metadata.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventEnvelope<E> {
    /// Auto-incrementing event id.
    pub id: usize,
    /// Event timestamp.
    pub created_at: DateTime<FixedOffset>,
    /// Aggregate type identifier.
    pub aggregate_type: String,
    /// Aggregate instance identifier.
    pub aggregate_id: String,
    /// Incrementing number unique where each aggregate instance starts from 0.
    pub sequence: usize,
    /// Event data
    pub event: E,
}

/// A unique identifier for an event type.
///
/// # Example
///
/// ```
/// # use thaloto::event::EventType;
/// #
/// enum BankAccountEvent {
///     DepositedFunds {
///         amount: f64,
///     },
///     WithdrewFunds {
///         amount: f64,
///     },
/// }
///
/// impl EventType for BankAccountEvent {
///     fn event_type(&self) -> &'static str {
///         use BankAccountEvent::*;
///
///         match self {
///             DepositedFunds { .. } => "DepositedFunds",
///             WithdrewFunds { .. } => "WithdrewFunds",
///         }
///     }
/// }
/// ```
pub trait EventType {
    /// Unique identifier for the active event variant.
    fn event_type(&self) -> &'static str;
}

/// An aggregate event envelope.
pub type AggregateEventEnvelope<A> = EventEnvelope<<A as Aggregate>::Event>;

/// A type which implements `IntoEvents` is used to convert into
/// a list of `Self::Event`.
///
/// Types returned from [`Aggregate`]'s typically implement this trait.
pub trait IntoEvents {
    /// Event type.
    type Event;

    /// Converts type into `Vec<Self::Event>`.
    fn into_events(self) -> Vec<Self::Event>;
}

impl<E> IntoEvents for Vec<E> {
    type Event = E;

    fn into_events(self) -> Vec<Self::Event> {
        self
    }
}

impl<E> IntoEvents for Option<E> {
    type Event = E;

    fn into_events(self) -> Vec<Self::Event> {
        self.map(|event| vec![event]).unwrap_or_default()
    }
}

impl<E, Err> IntoEvents for Result<E, Err>
where
    E: IntoEvents,
{
    type Event = <E as IntoEvents>::Event;

    fn into_events(self) -> Vec<Self::Event> {
        self.map(|event| event.into_events()).unwrap_or_default()
    }
}

impl<A, E> IntoEvents for (A, E)
where
    A: Aggregate,
    E: IntoEvents<Event = <A as Aggregate>::Event>,
{
    type Event = <A as Aggregate>::Event;

    fn into_events(self) -> Vec<Self::Event> {
        self.1.into_events()
    }
}
