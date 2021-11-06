use async_trait::async_trait;

use crate::{Aggregate, AggregateStateMutator, Error, Identity};

#[derive(Clone, Debug, PartialEq)]
pub struct AggregateEvent<'a> {
    pub aggregate_type: &'a str,
    pub aggregate_id: &'a str,
    pub event_type: &'a str,
    pub event_data: serde_json::Value,
}

#[async_trait]
pub trait Repository {
    async fn commit<A: Aggregate>(
        &self,
        events: Vec<<A as AggregateStateMutator>::Event>,
        agg: &mut A,
    ) -> Result<(), Error>
    where
        <A as Identity>::Identity: AsRef<str>;

    async fn save_events(&self, events: &[AggregateEvent]) -> Result<(), Error>;

    async fn load_aggregate<A: Aggregate>(&self, id: <A as Identity>::Identity)
        -> Result<A, Error>;
}
