use std::borrow::Cow;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use num_bigint::BigInt;
use scylla::batch::Batch;
use scylla::cql_to_rust::{FromCqlValError, FromRowError};
use scylla::frame::response::result::Row;
use scylla::prepared_statement::PreparedStatement;
use scylla::transport::errors::QueryError;
use scylla::transport::iterator::{NextRowError, TypedRowIterator};
use scylla::{FromRow, Session};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thalo::event_store::{Event, EventStore, NewEvent};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ScyllaEventStore {
    session: Arc<Session>,
    append_to_stream_stmt: PreparedStatement,
    iter_stream_stmt: PreparedStatement,
}

impl ScyllaEventStore {
    pub async fn new(session: Arc<Session>) -> Result<Self, QueryError> {
        session.query("CREATE KEYSPACE IF NOT EXISTS thalo WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1 }", ()).await?;
        session
            .query(
                r#"
                CREATE TABLE IF NOT EXISTS thalo.event_store (
                    stream_name text,
                    sequence bigint,
                    id uuid,
                    event_type text,
                    data blob,
                    timestamp timestamp,
                    PRIMARY KEY ((stream_name), sequence)
                ) WITH CLUSTERING ORDER BY (sequence ASC);    
            "#,
                (),
            )
            .await?;

        let append_to_stream_stmt = prepare_append_to_stream_stmt(&session).await?;
        let iter_stream_stmt = prepare_iter_stream_stmt(&session).await?;

        Ok(ScyllaEventStore {
            session: Arc::clone(&session),
            append_to_stream_stmt,
            iter_stream_stmt,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub stream_name: String,
    pub sequence: u64,
    pub id: Uuid,
    pub event_type: String,
    pub data: Value,
    pub timestamp: DateTime<Utc>,
}

impl Event for RecordedEvent {
    fn event_type(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.event_type)
    }

    fn data(&self) -> Cow<'_, Value> {
        Cow::Borrowed(&self.data)
    }

    fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl FromRow for RecordedEvent {
    fn from_row(row: Row) -> Result<Self, FromRowError> {
        let (stream_name, sequence, id, event_type, data, timestamp): (
            String,
            BigInt,
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
        let data = serde_json::from_slice(&data).map_err(|_err| FromRowError::BadCqlVal {
            err: FromCqlValError::BadVal,
            column: 4,
        })?;

        Ok(RecordedEvent {
            stream_name,
            sequence,
            id,
            event_type,
            data,
            timestamp,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    NextRow(#[from] NextRowError),
}

#[async_trait]
impl EventStore for ScyllaEventStore {
    type Event = RecordedEvent;
    type EventStream = EventStream;
    type Error = Error;

    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        let now = Utc::now();
        let mut batch = Batch::default();
        let mut batch_values = Vec::with_capacity(events.len());
        let events: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(i, new_event)| {
                let sequence = match expected_sequence {
                    Some(n) => n + 1 + i as u64,
                    None => i as u64,
                };
                RecordedEvent {
                    stream_name: stream_name.to_string(),
                    sequence,
                    id: Uuid::new_v4(),
                    event_type: new_event.event_type,
                    data: new_event.data,
                    timestamp: now,
                }
            })
            .collect();
        for event in &events {
            let data = serde_json::to_vec(&event.data).expect("Value shouldn't fail to serialize");
            batch.append_statement(self.append_to_stream_stmt.clone());
            batch_values.push((
                stream_name,
                event.sequence as i64,
                &event.id,
                &event.event_type,
                data,
                now,
            ));
        }

        self.session.batch(&batch, batch_values).await?;

        Ok(events)
    }

    async fn iter_stream(&self, stream_name: &str) -> Result<Self::EventStream, Self::Error> {
        let typed_row_iter = self
            .session
            .execute_iter(self.iter_stream_stmt.clone(), (stream_name,))
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

async fn prepare_append_to_stream_stmt(session: &Session) -> Result<PreparedStatement, QueryError> {
    session
        .prepare(
            r#"
                INSERT INTO thalo.event_store (
                    stream_name,
                    sequence,
                    id,
                    event_type,
                    data,
                    timestamp
                ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .await
}

async fn prepare_iter_stream_stmt(session: &Session) -> Result<PreparedStatement, QueryError> {
    session
        .prepare(
            r#"
                SELECT
                    stream_name,
                    sequence,
                    id,
                    event_type,
                    data,
                    timestamp
                FROM thalo.event_store
                WHERE stream_name = ?
                ORDER BY sequence ASC
            "#,
        )
        .await
}
