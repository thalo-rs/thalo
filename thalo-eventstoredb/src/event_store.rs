#![deny(missing_docs)]
use std::{fmt::Debug, vec};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eventstore::{
    AppendToStreamOptions, Client, EventData, ExpectedRevision, ReadAllOptions, ReadStreamOptions,
    StreamPosition, ResolvedEvent,
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

/// Event payload for Event Store (ESDB) event store implementation
#[derive(Serialize, Deserialize, Debug)]
pub struct ESDBEventPayload {
    created_at: DateTime<Utc>,
    aggregate_type: String,
    aggregate_id: String,
    event_data: serde_json::Value,
}

impl ESDBEventPayload {
    /// Convert event payload into AggregateEventEnvelope<A> for use in Thalo
    pub fn into_event_envelope<A>(self, id: usize) -> Result<AggregateEventEnvelope<A>, Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        Ok(AggregateEventEnvelope::<A> {
            id,
            created_at: self.created_at.into(),
            aggregate_type: self.aggregate_type.to_string(),
            aggregate_id: self.aggregate_id,
            sequence: id,
            event: serde_json::from_value(self.event_data)
                .map_err(Error::DeserializeEvent)?,
        })
    }
}

/// An event store backed by the Event Store Database (aliased ESDB).
/// https://www.eventstore.com/eventstoredb
///
/// See [crate] documentation for more info.
#[derive(Clone)]
pub struct ESDBEventStore {
    /// Event Store (ESDB) client instance
    client: Client,
}

impl ESDBEventStore {
    /// Creates an event store from an ESDB client
    pub fn new(client: Client) -> Self {
        ESDBEventStore { client }
    }

    fn stream_id<A>(&self, id: Option<&<A as Aggregate>::ID>) -> String
    where
        A: Aggregate,
    {
        let mut stream = <A as TypeId>::type_id().to_owned();

        if let Some(id_str) = id {
            stream.push('-');
            stream.push_str(&id_str.to_string());
        }

        stream
    }
}

impl ESDBEventStore {
    async fn read_stream(
        &self,
        stream_id: String,
        options: ReadStreamOptions
    ) -> Result<Vec<ResolvedEvent>, Error> {
        let mut stream = self
            .client
            .read_stream(stream_id, &options)
            .map_err(Error::ReadStreamError)
            .await?;

        let mut rv = vec![];
        loop {
            match stream.next().await {
                Ok(Some(event)) => {
                    rv.push(event);
                }
                Ok(None) => {
                    break;
                }
                Err(eventstore::Error::ResourceNotFound) => return Ok(vec![]),
                Err(e) => return Err(Error::ReadStreamError(e)),
            }
        }
        
        Ok(rv)
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
        let options = ReadStreamOptions::default()
            .position(StreamPosition::Start)
            .forwards();

        let events = self.read_stream(self.stream_id::<A>(id), options).await?;

        let mut rv = vec![];

        for event in events.iter() {
            let event_data = event.get_original_event();

            let event_payload = event_data
                .as_json::<ESDBEventPayload>()
                .map_err(Error::DeserializeEvent)?
                .into_event_envelope::<A>(event_data.revision as usize)?;

            rv.push(event_payload);
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
        let options = ReadAllOptions::default()
            .position(StreamPosition::Start)
            .forwards();

        let mut result = self
            .client
            .read_all(&options)
            .await
            .map_err(Error::ReadStreamError)?;

        let mut rv: Vec<AggregateEventEnvelope<A>> = vec![];

        while let Some(event) = result.next().await? {
            let event_data = event.get_original_event();

            if event_data.event_type.starts_with('$') {
                continue;
            }

            let is_aggregate_event = event_data
                .stream_id
                .starts_with(self.stream_id::<A>(None).as_str());

            if !is_aggregate_event {
                continue;
            }

            let sequence = event_data.revision as usize;
            if ids.contains(&sequence) {
                let event_payload = event_data
                    .as_json::<ESDBEventPayload>()
                    .map_err(Error::DeserializeEvent)?
                    .into_event_envelope::<A>(sequence)?;

                rv.push(event_payload);
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
        let options = ReadStreamOptions::default()
            .position(StreamPosition::End)
            .max_count(1);

        let events = self.read_stream(self.stream_id::<A>(Some(id)), options).await?;
        if let Some(event) = events.get(0) {
            let event_data = event.get_original_event();
            return Ok(Some(event_data.revision as usize));
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

        let sequence = self.load_aggregate_sequence::<A>(id).await.unwrap_or(None);

        let revision = {
            match sequence {
                Some(n) => ExpectedRevision::Exact(n as u64),
                None => ExpectedRevision::NoStream,
            }
        };

        let payload = events
            .iter()
            .map(|event| {
                let event_data_payload = ESDBEventPayload {
                    created_at: Utc::now(),
                    aggregate_type: <A as TypeId>::type_id().to_string(),
                    aggregate_id: id.to_string(),
                    event_data: serde_json::to_value(event).map_err(Error::SerializeEvent)?,
                };

                let event_data = EventData::json(event.event_type(), &event_data_payload)
                    .map_err(Error::SerializeEventDataPayload)?
                    .id(Uuid::new_v4());

                Result::<_, Error>::Ok(event_data)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let options = AppendToStreamOptions::default().expected_revision(revision);
        let res = self
            .client
            .append_to_stream(&self.stream_id::<A>(Some(id)), &options, payload)
            .map_err(|err| Error::WriteStreamError(sequence.unwrap_or(0) as usize, err))
            .await?;

        let end_position = res.next_expected_version as usize;
        let start_position = end_position - (events.len() - 1);

        Ok((start_position..end_position + 1).collect())
    }
}

#[cfg(feature = "debug")]
impl ESDBEventStore {
    /// Print the event store as a table to stdout.
    pub async fn print<A>(&self)
    where
        A: Aggregate,
        <A as Aggregate>::Event: serde::Serialize,
    {
        let stream_res = self.client.read_all(&Default::default()).await;

        let mut events: Vec<(Uuid, u64, ESDBEventPayload)> = vec![];
        if let Ok(mut stream) = stream_res {
            while let Some(event) = stream.next().await.unwrap() {
                let event_data = event.get_original_event();

                if event_data.event_type.starts_with('$') {
                    continue;
                }

                let is_aggregate_event = event_data
                    .stream_id
                    .starts_with(self.stream_id::<A>(None).as_str());

                if !is_aggregate_event {
                    continue;
                }

                events.push((
                    event_data.id,
                    event_data.revision,
                    event_data.as_json::<ESDBEventPayload>().unwrap(),
                ));
            }
        }

        let mut table = prettytable::Table::new();
        table.set_titles(
            [
                "ID",
                "Created At",
                "Aggregate Type",
                "Aggregate ID",
                "Sequence",
                "Event Data",
            ]
            .into(),
        );

        if events.is_empty() {
            table.add_row(["", "", "", "", "", ""].into());
        } else {
            for (uuid, revision, event) in events.iter() {
                table.add_row(
                    [
                        uuid.to_string(),
                        event.created_at.to_string(),
                        event.aggregate_type.to_string(),
                        event.aggregate_id.clone(),
                        revision.to_string(),
                        serde_json::to_string(&event.event_data).unwrap(),
                    ]
                    .into(),
                );
            }
        }

        table.printstd();
    }
}
