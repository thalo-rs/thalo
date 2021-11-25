use std::fmt;

use async_trait::async_trait;

use crate::{Aggregate, AggregateEvent, AggregateType, Error};

pub trait Event:
    serde::de::DeserializeOwned + serde::ser::Serialize + Clone + fmt::Debug + PartialEq + Send + Sync
{
    type Aggregate: Aggregate<Event = Self>;

    fn event_type(&self) -> &'static str;

    fn aggregate_event<'a>(&'a self, aggregate_id: &'a str) -> AggregateEvent<'a, Self::Aggregate>;
}

pub trait EventIdentity {
    fn event_type() -> &'static str;
}

pub trait CombinedEvent:
    serde::de::DeserializeOwned + serde::ser::Serialize + Clone + fmt::Debug + PartialEq + Send + Sync
{
    fn aggregate_types() -> Vec<&'static str>;
}

impl<E: Event> CombinedEvent for E {
    fn aggregate_types() -> Vec<&'static str> {
        vec![<E as Event>::Aggregate::aggregate_type()]
    }
}

pub trait EventView<E> {
    fn view(&self) -> Result<&E, Error>;

    fn view_opt(&self) -> Option<&E>;
}

/// EventHandler must run once only when multiple nodes of the
/// application are running at the same time (via locks in the database).
///
/// They keep track of their latest sequence and only process events that
/// have not yet been processed yet.
#[async_trait]
pub trait EventHandler {
    type Event: CombinedEvent;
    type View: Send;

    /// Handle an event and return an updated view.
    ///
    /// ```
    async fn handle(
        &mut self,
        id: String,
        event: Self::Event,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<Self::View, Error>;

    /// Commits an event handler from a previously handled view.
    ///
    /// Commit is where you would save/update the view, or just update the `last_event_id` and `last_sequence_id`s.
    ///
    /// # Example
    ///
    /// ```no_run
    /// async fn commit(
    ///     &mut self,
    ///     id: &str,
    ///     view: Option<UserView>,
    ///     event_id: i64,
    ///     event_sequence: i64,
    /// ) -> Result<(), Error> {
    ///     if let Some(view) = view {
    ///         self.repository
    ///             .save(&view, event_id, event_sequence)
    ///             .await?;
    ///     } else {
    ///         self.repository
    ///             .update_last_event(id, event_id, event_sequence)
    ///             .await?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    async fn commit(
        &mut self,
        _id: &str,
        _view: Self::View,
        _event_id: i64,
        _event_sequence: i64,
    ) -> Result<(), Error> {
        Ok(())
    }
}
