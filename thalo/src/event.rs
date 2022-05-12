//! Events

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::aggregate::Aggregate;

#[cfg(feature = "macros")]
pub use thalo_macros::{Event, EventType};

/// An event with additional metadata.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventEnvelope<E> {
    /// Auto-incrementing event id.
    pub id: u64,
    /// Event timestamp.
    pub created_at: DateTime<FixedOffset>,
    /// Aggregate type identifier.
    pub aggregate_type: String,
    /// Aggregate instance identifier.
    pub aggregate_id: String,
    /// Incrementing number unique where each aggregate instance starts from 0.
    pub sequence: u64,
    /// Event data
    pub event: E,
}

impl<E> EventEnvelope<E> {
    /// Map an event to a new type whilst preserving the rest of the envelope data.
    pub fn map_event<EE, F>(self, f: F) -> EventEnvelope<EE>
    where
        F: FnOnce(E) -> EE,
    {
        EventEnvelope {
            id: self.id,
            created_at: self.created_at,
            aggregate_type: self.aggregate_type,
            aggregate_id: self.aggregate_id,
            sequence: self.sequence,
            event: f(self.event),
        }
    }
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
/// #     async fn event_already_handled(&self, id: u64) -> bool {
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

/// A single event, typically returned by a command.
///
/// It is recommended to derive [thalo_macros::Event] and return the event directly
/// rather than use this type.
pub struct SingleEvent<E>(E);

impl<E> SingleEvent<E> {
    /// Inner event value.
    pub fn into_inner(self) -> E {
        self.0
    }
}

impl<E> AsRef<E> for SingleEvent<E> {
    fn as_ref(&self) -> &E {
        &self.0
    }
}

impl<E> AsMut<E> for SingleEvent<E> {
    fn as_mut(&mut self) -> &mut E {
        &mut self.0
    }
}

/// A type which implements `IntoEvents` is used to convert into
/// a list of `Self::Event`.
///
/// Types returned from [`Aggregate`]'s typically implement this trait.
pub trait IntoEvents<E> {
    /// Converts type into `Vec<Self::Event>`.
    fn into_events(self) -> Vec<E>;
}

impl<E, T> IntoEvents<E> for Vec<T>
where
    T: Into<E>,
{
    fn into_events(self) -> Vec<E> {
        self.into_iter().map(|event| event.into()).collect()
    }
}

impl<E, T> IntoEvents<E> for Option<T>
where
    T: Into<E>,
{
    fn into_events(self) -> Vec<E> {
        self.map(|event| vec![event.into()]).unwrap_or_default()
    }
}

impl<E, T, Err> IntoEvents<E> for Result<T, Err>
where
    T: IntoEvents<E>,
{
    fn into_events(self) -> Vec<E> {
        self.map(|event| event.into_events()).unwrap_or_default()
    }
}

impl<E, T> IntoEvents<E> for SingleEvent<T>
where
    T: Into<E>,
{
    fn into_events(self) -> Vec<E> {
        vec![self.0.into()]
    }
}
