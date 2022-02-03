use std::vec;

use async_trait::async_trait;
use chrono::Utc;
use eventstore::{
    All, AppendToStreamOptions, Client, ExpectedRevision, ReadStreamOptions, ResolvedEvent, Single,
    StreamPosition,
};
use serde::de::DeserializeOwned;
use thalo::{
    aggregate::{Aggregate, TypeId},
    event::{AggregateEventEnvelope, EventEnvelope},
    event_store::EventStore,
};

use crate::Error;

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

        let mut stream = self
            .client
            .read_stream(stream, &Default::default(), All)
            .await?;

        let mut rv: Vec<EventEnvelope<A::Event>> = vec![];

        while let Some(event) = stream.next().await? {
            let event_data_result = event
                .get_original_event()
                .as_json::<EventEnvelope<A::Event>>();

            if let Ok(data) = event_data_result {
                rv.push(data)
            }
        }

        return Ok(rv);
    }

    async fn load_events_by_id<A>(
        &self,
        ids: &[usize],
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

        let last_event = self.client.read_stream(&stream, &options, Single).await?;

        if let Ok(Some(ResolvedEvent {
            event,
            link,
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

        let res = self
            .client
            .append_to_stream(
                &stream,
                &options,
                events.iter().enumerate().map(|(index, event)| {
                    let created_at = Utc::now();
                    let aggregate_type = <A as TypeId>::type_id().to_string();
                    let aggregate_id = id.to_string();
                    let sequence = sequence.clone() + index;
                    let id = sequence.clone();
                    event_ids.push(id.clone());

                    AggregateEventEnvelope {
                        id,
                        aggregate_id,
                        aggregate_type,
                        sequence,
                        event,
                        created_at: created_at.into(),
                    }
                }),
            )
            .await;

        if let Ok(Ok(_)) = res {
            return Ok(event_ids);
        } else {
            return Err(Error::WrongExpectedVersion(sequence));
        }
    }
}
