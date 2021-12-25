use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use thaloto::{
    aggregate::{Aggregate, TypeId},
    event::AggregateEventEnvelope,
    event_store::EventStore,
};

use crate::Error;

/// An in memory event store.
///
/// This is useful for testing, but is not recommended
/// for production as the data does not persist to disk.
///
/// See [crate] documentation for more info.
#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    events: RwLock<Vec<EventRecord>>,
}

#[derive(Debug)]
struct EventRecord {
    created_at: DateTime<Utc>,
    aggregate_type: &'static str,
    aggregate_id: String,
    sequence: usize,
    event_data: serde_json::Value,
}

impl EventRecord {
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
            sequence: self.sequence,
            event: serde_json::from_value(self.event_data.clone())
                .map_err(Error::DeserializeEvent)?,
        })
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    type Error = Error;

    async fn load_events<A>(
        &self,
        id: Option<&<A as Aggregate>::ID>,
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let events_lock = self.events.read().map_err(|_| Error::RwPoison)?;

        let events = events_lock
            .iter()
            .enumerate()
            .filter(|(_index, event)| {
                event.aggregate_type == <A as TypeId>::type_id()
                    && id
                        .map(|id| event.aggregate_id == id.to_string())
                        .unwrap_or(true)
            })
            .map(|(index, event)| event.event_envelope::<A>(index))
            .collect::<Result<_, _>>()?;

        Ok(events)
    }

    async fn load_events_by_id<A>(
        &self,
        ids: &[usize],
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let events_lock = self.events.read().map_err(|_| Error::RwPoison)?;

        ids.iter()
            .filter_map(|id| {
                events_lock
                    .get(*id)
                    .map(|event| event.event_envelope::<A>(*id))
            })
            .collect::<Result<_, _>>()
    }

    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<Option<usize>, Self::Error>
    where
        A: Aggregate,
    {
        let events_lock = self.events.read().map_err(|_| Error::RwPoison)?;

        Ok(events_lock
            .iter()
            .filter_map(|event| {
                if event.aggregate_type == <A as TypeId>::type_id()
                    && event.aggregate_id == id.to_string()
                {
                    Some(event.sequence)
                } else {
                    None
                }
            })
            .max())
    }

    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<usize>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize,
    {
        if events.is_empty() {
            return Ok(vec![]);
        }

        let sequence = self.load_aggregate_sequence::<A>(id).await?;
        let mut event_ids = Vec::with_capacity(events.len());

        let mut events_lock = self.events.write().map_err(|_| Error::RwPoison)?;

        for (index, event) in events.iter().enumerate() {
            let created_at = Utc::now();
            let aggregate_type = <A as TypeId>::type_id();
            let aggregate_id = id.to_string();
            let sequence = sequence.map(|sequence| sequence + index + 1).unwrap_or(0);
            let event_data = serde_json::to_value(event).map_err(Error::SerializeEvent)?;

            events_lock.push(EventRecord {
                created_at,
                aggregate_type,
                aggregate_id,
                sequence,
                event_data,
            });
            event_ids.push(events_lock.len() - 1);
        }

        Ok(event_ids)
    }
}

#[cfg(feature = "debug")]
impl InMemoryEventStore {
    /// Print the event store as a table to stdout.
    pub fn print(&self) {
        let events_lock = self.events.read().unwrap();

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

        if events_lock.is_empty() {
            table.add_row(["", "", "", "", "", ""].into());
        } else {
            for (id, event) in events_lock.iter().enumerate() {
                table.add_row(
                    [
                        id.to_string(),
                        event.created_at.to_string(),
                        event.aggregate_type.to_string(),
                        event.aggregate_id.clone(),
                        event.sequence.to_string(),
                        serde_json::to_string(&event.event_data).unwrap(),
                    ]
                    .into(),
                );
            }
        }

        table.printstd();
    }
}
