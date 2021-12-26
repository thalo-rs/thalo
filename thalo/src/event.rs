//! Events

use async_trait::async_trait;
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
/// # use thalo::event::EventType;
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

/// A type which handles incoming events, typically used with an [`EventStream`](crate::event_stream::EventStream).
///
/// Projections in your system would implement `EventHandler` to update
/// the view in the database.
///
/// An implementation might do the following:
///
/// 1. Load from database.
/// 2. Verify event id has not already been handled (ensures _idempotency_).
/// 3. Apply updates to the state.
/// 4. Update model in database.
///
/// If your event would be considered a "first event" (`AccountOpened`, `GameStarted`, etc.),
/// then you may want to only apply your changes and insert the record if it does not already exist in the database.
///
/// # Examples
///
/// ```
/// use async_trait::async_trait;
/// # use thalo::event::{EventEnvelope, EventHandler};
/// # use thalo::tests_cfg::bank_account::BankAccountEvent;
/// #
/// # struct BankAccountProjection;
/// #
/// # impl BankAccountProjection {
/// #     async fn event_already_handled(&self, id: usize) -> bool {
/// #         false
/// #     }
/// #
/// #     async fn handle_opened_account(&self, id: String, balance: f64) -> Result<(), DbError> {
/// #         Ok(())
/// #     }
/// #
/// #     async fn handle_deposited_funds(&self, id: String, balance: f64) -> Result<(), DbError> {
/// #         Ok(())
/// #     }
/// #
/// #     async fn handle_withdrew_funds(&self, id: String, balance: f64) -> Result<(), DbError> {
/// #         Ok(())
/// #     }
/// # }
/// #
/// # struct DbError;
///
/// #[async_trait]
/// impl EventHandler<BankAccountEvent> for BankAccountProjection {
///     type Error = DbError;
///
///     async fn handle(
///         &self,
///         EventEnvelope {
///             id,
///             aggregate_id,
///             event,
///             ..
///         }: EventEnvelope<BankAccountEvent>
///     ) -> Result<(), Self::Error> {
///         use BankAccountEvent::*;
///
///         if self.event_already_handled(id).await {
///             // Event already handled, no need to perform any updates
///             return Ok(());
///         }
///
///         match event {
///             OpenedAccount { balance } => self.handle_opened_account(aggregate_id, balance).await?,
///             DepositedFunds { amount } => self.handle_deposited_funds(aggregate_id, amount).await?,
///             WithdrewFunds { amount } => self.handle_withdrew_funds(aggregate_id, amount).await?,
///         }
///
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait EventHandler<Event> {
    /// Error type returned by [`EventHandler::handle`].
    type Error;

    /// Handle an incoming event.
    async fn handle(&self, event: EventEnvelope<Event>) -> Result<(), Self::Error>;
}

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
