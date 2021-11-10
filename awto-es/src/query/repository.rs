use std::ops::Range;

use async_trait::async_trait;

use crate::{Aggregate, AggregateStateMutator, Error, Identity};

#[derive(Clone, Debug, PartialEq)]
pub struct AggregateEvent<'a> {
    pub aggregate_type: &'a str,
    pub aggregate_id: &'a str,
    pub event_type: &'a str,
    pub event_data: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AggregateEventOwned {
    pub id: i64,
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
        events: Vec<<A as AggregateStateMutator>::Event>,
        agg: &mut A,
    ) -> Result<(), Error>
    where
        <A as Identity>::Identity: AsRef<str>;

    /// Returns the last inserted event sequence
    async fn save_events(&self, events: &[AggregateEvent]) -> Result<Option<i64>, Error>;

    async fn load_aggregate<A: Aggregate>(&self, id: <A as Identity>::Identity)
        -> Result<A, Error>;

    async fn get_events(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        range: Range<i64>,
    ) -> Result<Vec<AggregateEventOwned>, Error>;
}

#[async_trait]
pub trait Repository<View, Id> {
    /// Insert or update view
    async fn save(&self, view: &View, event_sequence: i64) -> Result<(), Error>;

    /// Load an existing view
    async fn load(&self, id: &Id) -> Result<Option<(View, i64)>, Error>;

    /// Delete an existing view
    async fn delete(&self, id: &Id) -> Result<(), Error>;
}
