use std::{
    fs::{self, File},
    io::{self, BufRead, Write},
    path::Path,
    sync::Mutex,
};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use thalo::{aggregate::Aggregate, event::AggregateEventEnvelope, event_store::EventStore};
use thalo_inmemory::{EventRecord, InMemoryEventStore};

use crate::Error;

#[derive(Debug)]
pub struct FlatFileEventStore {
    pub event_store: InMemoryEventStore,
    file_store: Mutex<File>,
}

impl FlatFileEventStore {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file_store = fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;

        let event_store = InMemoryEventStore::default();
        {
            let mut events = event_store.events.write().map_err(|_| Error::RwPoison)?;

            let lines = io::BufReader::new(file_store.try_clone()?).lines();
            for line in lines.flatten() {
                let record: EventRecord =
                    serde_json::from_str(&line).map_err(Error::DeserializeEvent)?;
                events.push(record);
            }
        }

        Ok(FlatFileEventStore {
            event_store,
            file_store: Mutex::new(file_store),
        })
    }

    fn append_event(&self, record: &EventRecord) -> Result<(), Error> {
        let record_json = serde_json::to_string(record).map_err(Error::SerializeEvent)?;
        let mut file_guard = self.file_store.lock().map_err(|_| Error::RwPoison)?;
        writeln!(file_guard, "{}", record_json)?;

        Ok(())
    }
}

#[async_trait]
impl EventStore for FlatFileEventStore {
    type Error = Error;

    async fn load_events<A>(
        &self,
        id: Option<&<A as Aggregate>::ID>,
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        Ok(self.event_store.load_events::<A>(id).await?)
    }

    async fn load_events_by_id<A>(
        &self,
        ids: &[u64],
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        Ok(self.event_store.load_events_by_id::<A>(ids).await?)
    }

    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<Option<u64>, Self::Error>
    where
        A: Aggregate,
    {
        Ok(self.event_store.load_aggregate_sequence::<A>(id).await?)
    }

    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<u64>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize,
    {
        let event_ids = self.event_store.save_events::<A>(id, events).await?;

        let raw_events = self
            .event_store
            .events
            .read()
            .map_err(|_| Error::RwPoison)?;
        let event_records: Vec<_> = event_ids
            .iter()
            .filter_map(|event_id| raw_events.get(*event_id as usize))
            .collect();
        for event_record in event_records {
            self.append_event(event_record)?;
        }

        Ok(event_ids)
    }
}
