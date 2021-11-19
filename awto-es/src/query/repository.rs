use std::{marker::PhantomData, ops::Range};

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::{Aggregate, AggregateEventHandler, Error, Event, Projection};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AggregateEvent<'a, A: Aggregate> {
    aggregate: PhantomData<A>,
    pub aggregate_id: &'a str,
    pub event: &'a <A as Aggregate>::Event,
}

impl<'a, A> AggregateEvent<'a, A>
where
    A: Aggregate,
{
    pub fn new(aggregate_id: &'a str, event: &'a <A as Aggregate>::Event) -> Self {
        Self {
            aggregate: PhantomData,
            aggregate_id,
            event,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EventEnvelope<E> {
    pub id: i64,
    #[serde(with = "created_at_format")]
    pub created_at: DateTime<FixedOffset>,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i64,
    pub event: E,
}

#[async_trait]
pub trait EventStore {
    async fn commit<A: Aggregate>(
        &self,
        events: Vec<<A as AggregateEventHandler>::Event>,
        agg: &mut A,
    ) -> Result<(), Error>;

    /// Returns the last inserted event sequence
    async fn save_events<A: Aggregate>(
        &self,
        events: Vec<AggregateEvent<'_, A>>,
    ) -> Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error>;

    async fn get_aggregate_events<E: Event>(
        &self,
        aggregate_type: &str,
        aggregate_id: Option<&str>,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error>;

    async fn get_all_events<E: Event>(
        &self,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error>;

    async fn get_event_by_aggregate_sequence<A: Aggregate>(
        &self,
        sequence: i64,
    ) -> Result<Option<EventEnvelope<<A as Aggregate>::Event>>, Error>;

    async fn load_aggregate<A: Aggregate>(&self, id: String) -> Result<A, Error>;

    async fn resync_projection<P>(&self, projection: &mut P) -> Result<(), Error>
    where
        P: Projection + Send + Sync;
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
