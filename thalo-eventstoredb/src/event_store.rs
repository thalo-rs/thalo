#![deny(missing_docs)]
use std::{fmt::Debug, vec};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eventstore::{
    AppendToStreamOptions, Client, ClientSettings, EventData, ExpectedRevision, ReadAllOptions,
    ReadStreamOptions, ResolvedEvent, StreamPosition,
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

/// Event payload for EventStoreDB (ESDB) event store implementation.
#[derive(Serialize, Deserialize, Debug)]
pub struct EventPayload {
    created_at: DateTime<Utc>,
    aggregate_type: String,
    aggregate_id: String,
    event_data: serde_json::Value,
}

impl EventPayload {
    /// Convert event payload into AggregateEventEnvelope<A> for use in Thalo.
    pub fn into_event_envelope<A>(self, id: u64) -> Result<AggregateEventEnvelope<A>, Error>
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
            event: serde_json::from_value(self.event_data).map_err(Error::DeserializeEvent)?,
        })
    }
}

/// An event store backed by the EventStoreDB Database (aliased ESDB).
/// https://www.eventstore.com/eventstoredb
///
/// See [crate] documentation for more info.
#[derive(Clone)]
pub struct ESDBEventStore {
    /// EventStoreDB (ESDB) client instance
    client: Client,
}

impl ESDBEventStore {
    /// Creates an event store from an ESDB client.
    pub fn new(client: Client) -> Self {
        ESDBEventStore { client }
    }

    /// Creates an event store from a connection string.
    ///
    /// See [eventstore::grpc::ClientSettings].
    ///
    /// # Example
    ///
    /// ```
    /// ESDBEventStore::new_from_str("esdb://localhost:1234?tls=false")?;
    /// ```
    pub fn new_from_str(conn: &str) -> Result<Self, Error> {
        let client = Client::new(ClientSettings::parse_str(conn)?)?;
        Ok(ESDBEventStore { client })
    }

    /// Gets a reference to the event store client.
    pub fn client(&self) -> &Client {
        &self.client
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

    async fn read_stream(
        &self,
        stream_id: String,
        options: ReadStreamOptions,
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
            rv.push(resolved_event_to_event_envelope::<A>(event)?);
        }

        Ok(rv)
    }

    async fn load_events_by_id<A>(
        &self,
        ids: &[u64],
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

            let sequence = event_data.revision;
            if ids.contains(&sequence) {
                rv.push(resolved_event_to_event_envelope::<A>(&event)?);
            }
        }

        Ok(rv)
    }

    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<Option<u64>, Self::Error>
    where
        A: Aggregate,
    {
        let options = ReadStreamOptions::default()
            .position(StreamPosition::End)
            .max_count(1);

        let events = self
            .read_stream(self.stream_id::<A>(Some(id)), options)
            .await?;
        if let Some(event) = events.get(0) {
            let event_data = event.get_original_event();
            return Ok(Some(event_data.revision));
        }

        Ok(None)
    }

    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<u64>, Self::Error>
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

        let aggregate_id = id.to_string();

        let payload = events
            .iter()
            .map(|event| {
                let event_data = EventData::json(event.event_type(), &event)
                    .map_err(Error::SerializeEventDataPayload)?
                    .id(Uuid::new_v4())
                    .metadata_as_json(BorrowedEventMetadata {
                        created_at: &Utc::now(),
                        aggregate_type: <A as TypeId>::type_id(),
                        aggregate_id: &aggregate_id,
                    })
                    .map_err(Error::SerializeEventDataPayload)?;

                Result::<_, Error>::Ok(event_data)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let options = AppendToStreamOptions::default().expected_revision(revision);
        let res = self
            .client
            .append_to_stream(&self.stream_id::<A>(Some(id)), &options, payload)
            .map_err(|err| Error::WriteStreamError(sequence.unwrap_or(0), err))
            .await?;

        let end_position = res.next_expected_version;
        let start_position = end_position - (events.len() as u64 - 1);

        Ok((start_position..end_position + 1).collect())
    }
}

fn resolved_event_to_event_envelope<A>(
    resolved_event: &ResolvedEvent,
) -> Result<AggregateEventEnvelope<A>, Error>
where
    A: Aggregate,
    <A as Aggregate>::Event: DeserializeOwned,
{
    let event_data = resolved_event.get_original_event();

    let event = event_data
        .as_json::<<A as Aggregate>::Event>()
        .map_err(Error::DeserializeEvent)?;

    let metadata: EventMetadata =
        serde_json::from_slice(&event_data.custom_metadata).map_err(Error::ParseMetadata)?;

    Ok(EventEnvelope {
        id: event_data.revision,
        created_at: metadata.created_at.into(),
        aggregate_type: metadata.aggregate_type,
        aggregate_id: metadata.aggregate_id,
        sequence: event_data.revision,
        event,
    })
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EventMetadata {
    /// Event timestamp.
    pub created_at: DateTime<Utc>,
    /// Aggregate type identifier.
    pub aggregate_type: String,
    /// Aggregate instance identifier.
    pub aggregate_id: String,
}

#[derive(Serialize)]
struct BorrowedEventMetadata<'a> {
    pub created_at: &'a DateTime<Utc>,
    pub aggregate_type: &'a str,
    pub aggregate_id: &'a str,
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

        let mut events: Vec<(Uuid, u64, EventPayload)> = vec![];
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
                    event_data.as_json().unwrap(),
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
