use std::vec;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eventstore::{
    All, AppendToStreamOptions, Client, EventData, ExpectedRevision, ReadStreamOptions,
    Single, StreamPosition,
};
use futures::TryFutureExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thalo::{
    aggregate::{Aggregate, TypeId},
    event::{AggregateEventEnvelope, EventType},
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

impl ESDBEventPayload {
    fn event_envelope<A>(&self, id: usize) -> Result<AggregateEventEnvelope<A>, Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        Ok(AggregateEventEnvelope::<A> {
            id,
            created_at: self.created_at.into(),
            aggregate_type: self.aggregate_type.to_string(),
            aggregate_id: self.aggregate_id.clone(),
            sequence: id.clone(),
            event: serde_json::from_value(self.event_data.clone())
                .map_err(Error::DeserializeEvent)?,
        })
    }
}

pub struct ESDBEventStore {
    pub client: Client,
}

impl ESDBEventStore {
    pub fn new(client: Client) -> Self {
        ESDBEventStore { client }
    }
}

#[async_trait]
impl EventStore for ESDBEventStore {
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

        let mut rv: Vec<AggregateEventEnvelope<A>> = vec![];
        let result = self
            .client
            .read_stream(stream, &Default::default(), All)
            .await;

        if let Ok(mut stream) = result {
            while let Some(event) = stream.next().await? {
                let event_data = event.get_original_event();

                // TODO: - can we try event eventlope ids as uuid in addition to usize?
                // let uuid = event_data.id.clone();
                let sequence = usize::try_from(event_data.revision).unwrap();
                let event_payload = event_data
                    .as_json::<ESDBEventPayload>()
                    .map_err(Error::DeserializeEvent)?
                    .event_envelope::<A>(sequence)?;

                rv.push(event_payload);
            }
        }

        Ok(rv)
    }

    async fn load_events_by_id<A>(
        &self,
        ids: &[usize],
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let stream = <A as TypeId>::type_id().to_owned();
        let mut rv: Vec<AggregateEventEnvelope<A>> = vec![];

        let result = self
            .client
            .read_stream(stream, &Default::default(), All)
            .await;

        if let Ok(mut stream) = result {
            while let Some(event) = stream.next().await? {
                let event_data = event.get_original_event();

                // TODO: - can we try event eventlope ids as uuid in addition to usize?
                // let uuid = event_data.id.clone();
                let sequence = usize::try_from(event_data.revision).unwrap();
                if ids.contains(&sequence) {
                    let event_payload = event_data
                        .as_json::<ESDBEventPayload>()
                        .map_err(Error::DeserializeEvent)?
                        .event_envelope::<A>(sequence)?;

                    rv.push(event_payload);
                }
            }
        }

        Ok(rv)
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

        let last_event = self
            .client
            .read_stream(&stream, &options, Single)
            .map_err(Error::ReadStreamError)
            .await?;

        if let Ok(Some(event)) = last_event {
            let event_data = event.get_original_event();
            Ok(Some(usize::try_from(event_data.revision).unwrap()))
        } else {
            Ok(None)
        }
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

        let sequence = sequence.unwrap_or(0);
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

            let event_data = EventData::json(event.event_type(), &event_data_payload)
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
