use std::vec;

use async_trait::async_trait;
use eventstore::{All, Client, ClientSettings};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use thalo::{
    aggregate::Aggregate,
    event::{AggregateEventEnvelope, EventEnvelope},
    event_store::EventStore,
};

// pub enum Error {
//     ESDBError,
// }

// #[derive(Debug, Deserialize, Serialize)]
// pub struct EventRecord {
//     created_at: DateTime<Utc>,
//     aggregate_type: String,
//     aggregate_id: String,
//     sequence: usize,
//     event_data: serde_json::Value,
// }

// #[derive(Debug)]
pub struct EventStoreDBEventStore {
    /// Raw events stored in memory.
    pub client: Client,
    pub stream: String,
}

impl EventStoreDBEventStore {
    pub async fn new(settings: ClientSettings, stream: String) -> Self {
        let client = Client::create(settings).await.unwrap();

        EventStoreDBEventStore { client, stream }
    }
}

#[async_trait]
impl EventStore for EventStoreDBEventStore {
    type Error = eventstore::Error;

    async fn load_events<A>(
        &self,
        id: Option<&<A as Aggregate>::ID>,
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let result = self
            .client
            .read_stream(self.stream, &Default::default(), All)
            .await?;

        let mut rv: Vec<EventEnvelope<A::Event>> = vec![];

        if let Some(mut events) = result.ok() {
            while let Some(event) = events.try_next().await? {
                let event_data_result = event
                    .get_original_event()
                    .as_json::<EventEnvelope<A::Event>>();
                match event_data_result {
                    Ok(data) => rv.push(data),
                    _ => (),
                }
            }
        }

        return Ok(rv);
    }
}
