//! Event store

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    aggregate::Aggregate,
    event::{AggregateEventEnvelope, EventEnvelope},
};

/// Used to store & load events.
#[async_trait]
pub trait EventStore {
    /// The error type.
    type Error;

    /// Load events for a given aggregate.
    ///
    /// Query is filtered by aggregate id and sequence.
    async fn load_events<A>(
        &self,
        id: Option<&<A as Aggregate>::ID>,
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned;

    /// Load events by ids.
    async fn load_event_by_id<A>(
        &self,
        id: usize,
    ) -> Result<Option<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        Ok(self.load_events_by_id::<A>(&[id]).await?.into_iter().next())
    }

    /// Load events by ids.
    async fn load_events_by_id<A>(
        &self,
        ids: &[usize],
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned;

    /// Loads an aggregate by replaying all events.
    async fn load_aggregate<A>(&self, id: <A as Aggregate>::ID) -> Result<Option<A>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let events = self.load_events::<A>(Some(&id)).await?;
        if events.is_empty() {
            return Ok(None);
        }

        let mut aggregate = <A as Aggregate>::new(id);
        for EventEnvelope { event, .. } in events {
            aggregate.apply(event);
        }

        Ok(Some(aggregate))
    }

    /// Loads an aggregates latest sequence.
    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<Option<usize>, Self::Error>
    where
        A: Aggregate;

    /// Saves events for a given aggregate instance.
    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<usize>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize;
}
