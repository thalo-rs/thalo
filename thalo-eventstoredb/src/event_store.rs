use std::vec;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eventstore::{
    All, AppendToStreamOptions, Client, EventData, ExpectedRevision, ReadStreamOptions,
    ResolvedEvent, Single, StreamPosition
};
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thalo::{
    aggregate::{Aggregate, TypeId},
    event::{AggregateEventEnvelope, EventEnvelope, EventType},
    event_store::EventStore,
};
use uuid::Uuid;

use crate::Error;

#[derive(Serialize, Deserialize)]
struct ESDBEventPayload {
    created_at: DateTime<Utc>,
    aggregate_type: String,
    aggregate_id: String,
    event_data: serde_json::Value,
}

pub struct EventStoreDBEventStore {
    pub client: Client,
}

impl EventStoreDBEventStore {
    pub fn new(client: Client) -> Self {
        EventStoreDBEventStore { client }
    }
}

#[async_trait]
impl EventStore for EventStoreDBEventStore {
    type Error = Error;

    async fn load_events<A>(
        &self,
        id: Option<&<A as Aggregate>::ID>,
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
    <A as Aggregate>::Event: DeserializeOwned,
    {
        let mut stream = <A as TypeId>::type_id().to_owned();

        if let Some(id_str) = id {
            stream.push_str(&String::from(":"));
            stream.push_str(&id_str.to_string());
        }

        let mut rv: Vec<EventEnvelope<A::Event>> = vec![];
        let result = self
            .client
            .read_stream(stream, &Default::default(), All)
            .await;

        if let Ok(mut stream) = result {
            while let Some(event) = stream.next().await? {
                let event_data_result = event
                    .get_original_event()
                    .as_json::<EventEnvelope<A::Event>>();

                if let Ok(data) = event_data_result {
                    rv.push(data)
                }
            }
        }

        return Ok(rv);
    }

    async fn load_events_by_id<A>(
        &self,
        _ids: &[usize],
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
    <A as Aggregate>::Event: DeserializeOwned,
    {
        todo!()
    }

    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<Option<usize>, Self::Error>
    where
        A: Aggregate,
    {
        let mut stream = <A as TypeId>::type_id().to_owned();
        stream.push_str(&String::from(":"));
        stream.push_str(&id.to_string());

        let options = ReadStreamOptions::default().position(StreamPosition::End);

        let last_event = self.client.read_stream(&stream, &options, Single)
            .map_err(Error::ReadStreamError)
            .await?;

        if let Ok(Some(ResolvedEvent {
            event: _,
            link: _,
            commit_position: Some(commit_position),
        })) = last_event
        {
            if let Ok(s) = usize::try_from(commit_position) {
                return Ok(Some(s));
            }
        }

        Ok(None)
    }

    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<usize>, Self::Error>
    where
        A: Aggregate,
    <A as Aggregate>::Event: serde::Serialize,
    {
        if events.is_empty() {
            return Ok(vec![]);
        }

        let mut stream = <A as TypeId>::type_id().to_owned();
        stream.push_str(&String::from(":"));
        stream.push_str(&id.to_string());

        let sequence = self.load_aggregate_sequence::<A>(id).await?;
        let mut event_ids = Vec::with_capacity(events.len());

        let sequence = sequence.map(|sequence| sequence + 1).unwrap_or(0);
        let revision = u64::try_from(sequence).unwrap();
        let options =
            AppendToStreamOptions::default().expected_revision(ExpectedRevision::Exact(revision));

        let mut payload: Vec<EventData> = vec![];

        for (index, event) in events.iter().enumerate() {
            let created_at = Utc::now();
            let aggregate_type = <A as TypeId>::type_id().to_string();
            let aggregate_id = id.to_string();
            let sequence = sequence.clone() + index;
            let id = sequence.clone();
            let data = serde_json::to_value(event).map_err(Error::SerializeEvent)?;
            event_ids.push(id.clone());

            let event_data_payload = ESDBEventPayload {
                created_at,
                aggregate_type,
                aggregate_id,
                event_data: data,
            };

            let event_data =
                EventData::json(event.event_type(), &event_data_payload)
                .map_err(Error::SerializeEventDataPayload)?
                .id(Uuid::new_v4());

            payload.push(event_data);
        }

        let _ = self
            .client
            .append_to_stream(&stream, &options, payload)
            .map_err(|err| Error::WriteStreamError(sequence, err))
            .await?;

        Ok(event_ids)
    }
}
