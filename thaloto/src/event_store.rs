use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::aggregate::Aggregate;

/// Used to store & load events.
#[async_trait(?Send)]
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

    /// Loads an aggregate by replaying all events.
    async fn load_aggregate<A>(&self, id: <A as Aggregate>::ID) -> Result<A, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let events = self.load_events::<A>(Some(&id)).await?;

        let mut aggregate = <A as Aggregate>::new(id);
        for EventEnvelope { event, .. } in events {
            aggregate.apply(event);
        }

        Ok(aggregate)
    }

    /// Loads an aggregates latest sequence.
    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<i64, Self::Error>
    where
        A: Aggregate;

    /// Saves events for a given aggregate instance.
    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[&<A as Aggregate>::Event],
    ) -> Result<Vec<i64>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize;
}

/// An aggregate event envelope.
pub type AggregateEventEnvelope<A> = EventEnvelope<<A as Aggregate>::Event>;

/// A stored event with additional metadata.
#[derive(Debug, Deserialize, Serialize)]
pub struct EventEnvelope<E> {
    /// Auto-incrementing event id.
    pub id: i64,
    /// Event timestamp.
    #[serde(with = "created_at_format")]
    pub created_at: DateTime<FixedOffset>,
    /// Aggregate type identifier.
    pub aggregate_type: String,
    /// Aggregate instance identifier.
    pub aggregate_id: String,
    /// Incrementing number unique where each aggregate instance starts from 0.
    pub sequence: i64,
    /// Event data
    pub event: E,
}

mod created_at_format {
    use chrono::{DateTime, FixedOffset};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT_SER: &str = "%F %T.%f%z";
    const FORMAT_DE: &str = "%F %T.%f%#z";

    pub fn serialize<S>(date: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT_SER));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_str(&s, FORMAT_DE).map_err(serde::de::Error::custom)
    }
}
