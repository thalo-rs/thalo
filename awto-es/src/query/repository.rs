use std::ops::Range;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::{Aggregate, AggregateEventHandler, Error, Projection};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AggregateEvent<'a> {
    pub aggregate_type: &'a str,
    pub aggregate_id: &'a str,
    pub event_type: &'a str,
    pub event_data: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AggregateEventOwned {
    pub id: i64,
    #[serde(with = "created_at_format")]
    pub created_at: DateTime<FixedOffset>,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i64,
    pub event_type: String,
    pub event_data: serde_json::Value,
}

#[async_trait]
pub trait EventStore {
    async fn commit<A: Aggregate>(
        &self,
        events: Vec<<A as AggregateEventHandler>::Event>,
        agg: &mut A,
    ) -> Result<(), Error>;

    /// Returns the last inserted event sequence
    async fn save_events(
        &self,
        events: &[AggregateEvent],
    ) -> Result<Vec<AggregateEventOwned>, Error>;

    async fn get_aggregate_events(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        range: Range<i64>,
    ) -> Result<Vec<AggregateEventOwned>, Error>;

    async fn get_all_events(&self, range: Range<i64>) -> Result<Vec<AggregateEventOwned>, Error>;

    async fn get_event_by_aggregate_sequence<A: Aggregate>(
        &self,
        sequence: i64,
    ) -> Result<Option<AggregateEventOwned>, Error>;

    async fn load_aggregate<A: Aggregate>(&self, id: String) -> Result<A, Error>;

    async fn resync_projection<P>(&self, projection: &mut P) -> Result<(), Error>
    where
        P: Projection + Send + Sync;
}

#[async_trait]
pub trait Repository<View> {
    /// Insert or update view
    async fn save(&self, view: &View, event_id: i64, event_sequence: i64) -> Result<(), Error>;

    /// Load an existing view
    async fn load(&self, id: &str) -> Result<Option<(View, i64)>, Error>;

    /// Delete an existing view
    async fn delete(&self, id: &str) -> Result<(), Error>;

    /// Load the latest event version number
    async fn last_event_id(&self) -> Result<Option<i64>, Error>;

    /// Load the latest event sequence number from an aggregate
    async fn last_event_sequence(&self, id: &str) -> Result<Option<i64>, Error>;
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
