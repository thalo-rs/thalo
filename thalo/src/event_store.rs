//! Event store

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    aggregate::Aggregate,
    event::{AggregateEventEnvelope, EventEnvelope, IntoEvents},
};

/// Used to store & load events.
#[async_trait]
pub trait EventStore {
    /// The error type.
    type Error;

    /// Loads an aggregate by replaying all events.
    async fn execute<A, C, R>(&self, id: <A as Aggregate>::ID, cmd: C) -> Result<R, Self::Error>
    where
        A: Aggregate + Send + Sync,
        <A as Aggregate>::ID: Clone,
        <A as Aggregate>::Event: Clone + DeserializeOwned + Serialize,
        C: (FnOnce(&A) -> R) + Send,
        R: Clone + IntoEvents<<A as Aggregate>::Event> + Send,
    {
        let aggregate = self
            .load_aggregate::<A>(id.clone())
            .await?
            .unwrap_or_else(|| A::new(id));

        let result = cmd(&aggregate);
        let events = result.clone().into_events();

        self.save_events::<A>(aggregate.id(), &events).await?;

        Ok(result)
    }

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
        id: u64,
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
        ids: &[u64],
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
    ) -> Result<Option<u64>, Self::Error>
    where
        A: Aggregate;

    /// Saves events for a given aggregate instance.
    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<u64>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize;
}
