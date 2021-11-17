use async_trait::async_trait;

use crate::{Aggregate, AggregateEvent, Error};

pub trait Event: serde::de::DeserializeOwned + serde::ser::Serialize + Clone + Send + Sync {
    type Aggregate: Aggregate;

    fn event_type(&self) -> &'static str;

    fn aggregate_event<'a>(&self, aggregate_id: &'a str) -> Result<AggregateEvent<'a>, Error>;
}

/// EventHandler must run once only when multiple nodes of the
/// application are running at the same time (via locks in the database).
///
/// They keep track of their latest sequence and only process events that
/// have not yet been processed yet.
#[async_trait]
pub trait EventHandler {
    type Event: Event;

    async fn handle(
        &mut self,
        id: String,
        event: Self::Event,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error>;
}
