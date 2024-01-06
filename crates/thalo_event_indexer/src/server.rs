use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use chrono::{NaiveDateTime, TimeZone, Utc};
use thalo::event_store::AppendStreamError;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::{EventIndexer, EventIndexerEventStore};

tonic::include_proto!("event_indexer");

pub struct Server<S> {
    event_indexer: Arc<Mutex<EventIndexer<S>>>,
}

impl<S> Server<S> {
    pub fn new(event_indexer: Arc<Mutex<EventIndexer<S>>>) -> Self {
        Server { event_indexer }
    }
}

#[async_trait]
impl<S> event_indexer_server::EventIndexer for Server<S>
where
    S: EventIndexerEventStore + Send + Sync + 'static,
{
    async fn persist_events(
        &self,
        request: Request<PersistEventsRequest>,
    ) -> tonic::Result<Response<PersistEventsReply>> {
        let PersistEventsRequest {
            stream_name,
            events,
            expected_sequence,
        } = request.into_inner();
        let events = events
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<anyhow::Result<_>>()
            .map_err(|err| Status::invalid_argument(err.to_string()))?;
        let mut event_indexer = self.event_indexer.lock().await;

        let global_sequence = event_indexer
            .process_events(stream_name, events, expected_sequence)
            .await
            .map_err(|err| match err {
                AppendStreamError::Error(err) => Status::internal(err.to_string()),
                err => Status::aborted(err.to_string()),
            })?;

        Ok(Response::new(PersistEventsReply { global_sequence }))
    }
}

impl<S> Clone for Server<S> {
    fn clone(&self) -> Self {
        Server {
            event_indexer: Arc::clone(&self.event_indexer),
        }
    }
}

impl TryFrom<Event> for crate::Event {
    type Error = anyhow::Error;

    fn try_from(ev: Event) -> Result<Self, Self::Error> {
        let id = ev.id.parse()?;
        let timestamp = TimeZone::from_utc_datetime(
            &Utc,
            &NaiveDateTime::from_timestamp_millis(ev.timestamp).context("invalid timestamp")?,
        );

        Ok(crate::Event {
            id,
            event_type: ev.event_type,
            data: ev.data,
            timestamp,
        })
    }
}
