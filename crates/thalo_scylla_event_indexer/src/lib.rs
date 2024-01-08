use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use scylla::batch::{Batch, BatchType};
use scylla::cql_to_rust::{FromCqlValError, FromRowError};
use scylla::frame::response::result::Row;
use scylla::prepared_statement::PreparedStatement;
use scylla::statement::{Consistency, SerialConsistency};
use scylla::transport::errors::QueryError;
use scylla::transport::iterator::{NextRowError, TypedRowIterator};
use scylla::transport::query_result::{FirstRowError, FirstRowTypedError};
use scylla::{FromRow, Session};
use serde::{Deserialize, Serialize};
use server::event_indexer_client::EventIndexerClient;
use thalo::event_store::{AppendStreamError, EventData, EventStore, NewEvent, StreamEvent};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tonic::{Code, Request};
use uuid::Uuid;

const BUCKET_SIZE: i64 = 1_000;

pub mod server;

pub struct EventIndexer<S> {
    next_global_sequence: u64,
    event_store: S,
}

impl<S> EventIndexer<S>
where
    S: EventIndexerEventStore,
{
    pub async fn new(event_store: S) -> anyhow::Result<Self> {
        let next_global_sequence = event_store
            .latest_global_sequence()
            .await?
            .map(|seq| seq + 1)
            .unwrap_or(0);

        Ok(EventIndexer {
            next_global_sequence,
            event_store,
        })
    }

    pub async fn process_events(
        &mut self,
        stream_name: String,
        events: Vec<Event>,
        expected_sequence: Option<u64>,
    ) -> Result<Option<u64>, AppendStreamError<Error>> {
        let events_len = events.len();
        self.event_store
            .persist_events(
                stream_name,
                expected_sequence,
                self.next_global_sequence,
                events,
            )
            .await?;
        self.next_global_sequence += events_len as u64;

        Ok(self.next_global_sequence.checked_sub(1))
    }
}

pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub data: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

#[async_trait]
pub trait EventIndexerEventStore {
    async fn latest_global_sequence(&self) -> anyhow::Result<Option<u64>>;

    /// Persists events starting with the specified global_sequence.
    async fn persist_events(
        &self,
        stream_name: String,
        expected_sequence: Option<u64>,
        global_sequence: u64,
        events: Vec<Event>,
    ) -> Result<(), AppendStreamError<Error>>;
}

#[derive(Clone)]
pub struct ScyllaEventStore {
    session: Arc<Session>,
    append_query: PreparedStatement,
    latest_global_sequence_stmt: PreparedStatement,
    latest_global_sequence_in_events_stmt: PreparedStatement,
    latest_stream_sequence_stmt: PreparedStatement,
    iter_stream_stmt: PreparedStatement,
    latest_stream_sequence_cache: Arc<Mutex<HashMap<String, u64>>>,
    #[cfg(feature = "client")]
    client: EventIndexerClient<tonic::transport::Channel>,
}

impl ScyllaEventStore {
    pub async fn new(
        session: Arc<Session>,
        #[cfg(feature = "client")] addr: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let append_query = session
            .prepare(
                "
                    INSERT INTO thalo.events (
                        stream_name,
                        sequence,
                        global_sequence,
                        id,
                        event_type,
                        data,
                        timestamp,
                        bucket,
                        bucket_size
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    IF NOT EXISTS
                ",
            )
            .await?;
        let latest_global_sequence_stmt = session
            .prepare("SELECT MAX(global_sequence) FROM thalo.events_by_global_sequence")
            .await?;
        let latest_global_sequence_in_events_stmt = session
            .prepare("SELECT MAX(global_sequence) FROM thalo.events")
            .await?;
        let latest_stream_sequence_stmt = session
            .prepare("SELECT max(sequence) FROM thalo.events WHERE stream_name = ?")
            .await?;
        let iter_stream_stmt = session.prepare("SELECT stream_name, sequence, global_sequence, id, event_type, data, timestamp FROM thalo.events WHERE stream_name = ? AND sequence >= ? ORDER BY sequence ASC").await?;

        Ok(ScyllaEventStore {
            session,
            append_query,
            latest_global_sequence_stmt,
            latest_global_sequence_in_events_stmt,
            latest_stream_sequence_stmt,
            iter_stream_stmt,
            latest_stream_sequence_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "client")]
            client: EventIndexerClient::connect(addr.into()).await?,
        })
    }

    pub async fn is_consistent(&self) -> anyhow::Result<bool> {
        let latest_global_sequence_in_events = self.latest_global_sequence_in_events().await?;
        let latest_global_sequence_in_materialized_view = self.latest_global_sequence().await?;

        Ok(latest_global_sequence_in_events == latest_global_sequence_in_materialized_view)
    }

    pub async fn latest_global_sequence_in_events(&self) -> anyhow::Result<Option<u64>> {
        let seq = self
            .session
            .execute(&self.latest_global_sequence_in_events_stmt, ())
            .await?
            .first_row_typed::<(Option<i64>,)>()?
            .0
            .map(|seq| seq as u64);

        Ok(seq)
    }

    async fn latest_stream_sequence(&self, stream_name: &str) -> Result<Option<u64>, Error> {
        match self
            .latest_stream_sequence_cache
            .lock()
            .await
            .get(stream_name)
        {
            Some(seq) => Ok(Some(*seq)),
            None => Ok(self
                .session
                .execute(&self.latest_stream_sequence_stmt, (stream_name,))
                .await
                .map_err(Error::Query)?
                .first_row_typed::<(Option<i64>,)>()
                .map_err(Error::FirstRowTyped)?
                .0
                .map(|n| n as u64)),
        }
    }
}

#[async_trait]
impl EventIndexerEventStore for ScyllaEventStore {
    async fn latest_global_sequence(&self) -> anyhow::Result<Option<u64>> {
        let seq = self
            .session
            .execute(&self.latest_global_sequence_stmt, ())
            .await?
            .first_row_typed::<(Option<i64>,)>()?
            .0
            .map(|seq| seq as u64);

        Ok(seq)
    }

    async fn persist_events(
        &self,
        stream_name: String,
        expected_sequence: Option<u64>,
        global_sequence: u64,
        events: Vec<Event>,
    ) -> Result<(), AppendStreamError<Error>> {
        let events_len = events.len();
        let latest_sequence = self.latest_stream_sequence(&stream_name).await?;
        if expected_sequence != latest_sequence {
            return Err(AppendStreamError::WrongExpectedSequence {
                expected: expected_sequence,
                current: latest_sequence,
            });
        }

        let mut batch = Batch::new(BatchType::Logged);
        batch.set_consistency(Consistency::Quorum);
        batch.set_serial_consistency(Some(SerialConsistency::LocalSerial));
        batch.set_is_idempotent(true);
        let mut values = Vec::with_capacity(events.len());

        for (i, event) in events.into_iter().enumerate() {
            batch.append_statement(self.append_query.clone());

            let sequence = match expected_sequence {
                Some(n) => n as i64 + 1 + i as i64,
                None => i as i64,
            };
            let global_sequence = (global_sequence + i as u64) as i64;
            let bucket = global_sequence / BUCKET_SIZE;

            values.push((
                &stream_name,
                sequence,
                global_sequence,
                event.id,
                event.event_type,
                event.data,
                event.timestamp,
                bucket,
                BUCKET_SIZE,
            ));
        }

        let applied = self
            .session
            .batch(&batch, values)
            .await
            .map_err(Error::Query)?
            .first_row()
            .map_err(Error::FirstRow)?
            .columns
            .first()
            .and_then(|column| column.as_ref().and_then(|column| column.as_boolean()))
            .ok_or(Error::MissingAppliedColumn)?;
        if !applied {
            return Err(AppendStreamError::WriteConflict);
        }

        self.latest_stream_sequence_cache.lock().await.insert(
            stream_name,
            match expected_sequence {
                Some(n) => n + events_len as u64,
                None => events_len as u64 - 1,
            },
        );

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub stream_name: String,
    pub sequence: u64,
    pub global_sequence: u64,
    pub id: Uuid,
    pub event_type: String,
    pub data: EventData,
    pub timestamp: DateTime<Utc>,
}

impl StreamEvent for RecordedEvent {
    fn event_type(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.event_type)
    }

    fn data(&self) -> Cow<'_, EventData> {
        Cow::Borrowed(&self.data)
    }

    fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl FromRow for RecordedEvent {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        let (stream_name, sequence, global_sequence, id, event_type, data, timestamp): (
            String,
            i64,
            i64,
            Uuid,
            String,
            Vec<u8>,
            DateTime<Utc>,
        ) = FromRow::from_row(row)?;
        let sequence = sequence
            .try_into()
            .map_err(|_err| FromRowError::BadCqlVal {
                err: FromCqlValError::BadVal,
                column: 1,
            })?;
        let global_sequence =
            global_sequence
                .try_into()
                .map_err(|_err| FromRowError::BadCqlVal {
                    err: FromCqlValError::BadVal,
                    column: 2,
                })?;
        let data = serde_json::from_slice(&data).map_err(|_err| FromRowError::BadCqlVal {
            err: FromCqlValError::BadVal,
            column: 5,
        })?;

        Ok(RecordedEvent {
            stream_name,
            sequence,
            global_sequence,
            id,
            event_type,
            data,
            timestamp,
        })
    }
}

#[async_trait]
impl EventStore for ScyllaEventStore {
    type Event = RecordedEvent;
    type EventStream = EventStream;
    type Error = Error;

    #[cfg(feature = "client")]
    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, AppendStreamError<Self::Error>> {
        if events.is_empty() {
            return Ok(vec![]);
        }

        let now = Utc::now();
        let (event_data, events): (Vec<_>, Vec<_>) = events
            .into_iter()
            .map(|event| {
                let id = Uuid::new_v4();
                let data = event.data;
                let data_vec =
                    serde_json::to_vec(&data).expect("maps should not fail to serialize");

                (
                    (id, event.event_type, data),
                    server::Event {
                        id: id.to_string(),
                        event_type: event.event_type.to_string(),
                        data: data_vec,
                        timestamp: now.timestamp_millis(),
                    },
                )
            })
            .unzip();
        let res = self
            .client
            .clone()
            .persist_events(Request::new(server::PersistEventsRequest {
                stream_name: stream_name.to_string(),
                events,
                expected_sequence,
            }))
            .await;
        let server::PersistEventsReply { global_sequence } = match res {
            Ok(res) => res.into_inner(),
            Err(status) => {
                let err = match status.code() {
                    Code::Aborted => AppendStreamError::WriteConflict,
                    _ => AppendStreamError::Error(Error::Grpc(status.to_string())),
                };

                return Err(err);
            }
        };
        let global_sequence = global_sequence
            .expect("global sequence should be set if we've written at least one event");

        let recorded_events = event_data
            .into_iter()
            .enumerate()
            .map(|(i, (id, event_type, data))| {
                let sequence = match expected_sequence {
                    Some(n) => n + 1 + i as u64,
                    None => i as u64,
                };
                let global_sequence = global_sequence + i as u64;

                RecordedEvent {
                    stream_name: stream_name.to_string(),
                    sequence,
                    global_sequence,
                    id,
                    event_type: event_type.to_string(),
                    data,
                    timestamp: now,
                }
            })
            .collect();

        Ok(recorded_events)
    }

    #[cfg(not(feature = "client"))]
    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, AppendStreamError<Self::Error>> {
        unimplemented!("servers should not call append_to_stream")
    }

    async fn iter_stream(
        &self,
        stream_name: &str,
        from: u64,
    ) -> Result<Self::EventStream, Self::Error> {
        let typed_row_iter = self
            .session
            .execute_iter(self.iter_stream_stmt.clone(), (stream_name, from as i64))
            .await?
            .into_typed();

        Ok(EventStream::new(typed_row_iter))
    }
}

pub struct EventStream {
    inner: TypedRowIterator<RecordedEvent>,
}

impl EventStream {
    fn new(inner: TypedRowIterator<RecordedEvent>) -> Self {
        EventStream { inner }
    }
}

impl Stream for EventStream {
    type Item = Result<RecordedEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner_stream = Pin::new(&mut self.inner);
        let mapped = inner_stream.map(|row_result| match row_result {
            Ok(event) => Ok(event),
            Err(err) => Err(err.into()),
        });
        let pinned = std::pin::pin!(mapped);
        pinned.poll_next(cx)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    FirstRow(#[from] FirstRowError),
    #[error(transparent)]
    FirstRowTyped(#[from] FirstRowTypedError),
    #[error("{0}")]
    Grpc(String),
    #[error("missing [applied] column")]
    MissingAppliedColumn,
    #[error(transparent)]
    NextRow(#[from] NextRowError),
    #[error(transparent)]
    Query(#[from] QueryError),
}

impl From<Error> for AppendStreamError<Error> {
    fn from(err: Error) -> Self {
        AppendStreamError::Error(err)
    }
}
