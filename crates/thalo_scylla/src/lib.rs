use std::borrow::Cow;
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use scylla::batch::Batch;
use scylla::cql_to_rust::{FromCqlValError, FromRowError};
use scylla::frame::response::result::Row;
use scylla::prepared_statement::PreparedStatement;
use scylla::statement::{Consistency, SerialConsistency};
use scylla::transport::errors::QueryError;
use scylla::transport::iterator::{NextRowError, TypedRowIterator};
use scylla::transport::query_result::{FirstRowError, FirstRowTypedError};
use scylla::{FromRow, Session};
use serde::{Deserialize, Serialize};
use thalo::event_store::{AppendStreamError, EventData, EventStore, NewEvent, StreamEvent};
use thiserror::Error;
use uuid::Uuid;

const CREATE_KEYSPACE_QUERY: &str = include_str!("../cql/create_keyspace.sql");
const CREATE_EVENT_STORE_QUERY: &str = include_str!("../cql/create_event_store.sql");
const APPEND_TO_STREAM_QUERY: &str = include_str!("../cql/append_to_stream.sql");
const ITER_STREAM_QUERY: &str = include_str!("../cql/iter_stream.sql");
const MAX_SEQUENCE_QUERY: &str = include_str!("../cql/max_sequence.sql");

#[derive(Clone, Debug)]
pub struct ScyllaEventStore {
    session: Arc<Session>,
    append_to_stream_stmt: PreparedStatement,
    iter_stream_stmt: PreparedStatement,
    max_sequence_stmt: PreparedStatement,
}

impl ScyllaEventStore {
    pub async fn new(session: Arc<Session>) -> Result<Self, QueryError> {
        // session.query(CREATE_KEYSPACE_QUERY, ()).await?;
        // session.query(CREATE_EVENT_STORE_QUERY, ()).await?;

        // println!("prepare statements");
        let append_to_stream_stmt = session.prepare(APPEND_TO_STREAM_QUERY).await?;
        let iter_stream_stmt = session.prepare(ITER_STREAM_QUERY).await?;
        let max_sequence_stmt = session.prepare(MAX_SEQUENCE_QUERY).await?;

        Ok(ScyllaEventStore {
            session: Arc::clone(&session),
            append_to_stream_stmt,
            iter_stream_stmt,
            max_sequence_stmt,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub stream_name: String,
    pub sequence: u64,
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
        let (stream_name, sequence, id, event_type, data, timestamp): (
            String,
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
    FirstRow(#[from] FirstRowError),
    #[error(transparent)]
    FirstRowTyped(#[from] FirstRowTypedError),
    #[error("missing [applied] column")]
    MissingAppliedColumn,
    #[error(transparent)]
    NextRow(#[from] NextRowError),
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error("write conflict - something else wrote to this stream at the same time")]
    WriteConflict,
    #[error("wrong expected sequence")]
    WrongExpectedSequence,
}

impl From<Error> for AppendStreamError<Error> {
    fn from(err: Error) -> Self {
        AppendStreamError::Error(err)
    }
}

#[async_trait]
impl EventStore for ScyllaEventStore {
    type Event = RecordedEvent;
    type EventStream = EventStream;
    type Error = Error;

    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, AppendStreamError<Self::Error>> {
        let max_sequence = self
            .session
            .execute(&self.max_sequence_stmt, (stream_name,))
            .await
            .map_err(Error::Query)?
            .first_row_typed::<(Option<i64>,)>()
            .map_err(Error::FirstRowTyped)?
            .0
            .map(|n| n as u64);
        if expected_sequence != max_sequence {
            return Err(AppendStreamError::WrongExpectedSequence {
                expected: expected_sequence,
                current: max_sequence,
            });
        }

        let now = Utc::now();
        let mut batch = Batch::default();
        batch.set_consistency(Consistency::Quorum);
        batch.set_serial_consistency(Some(SerialConsistency::Serial));
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
                    event_type: new_event.event_type.to_string(),
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
                event.sequence as i64,
                &event.id,
                &event.event_type,
                data,
                now,
                event.sequence as i64 / 1000,
            ));
        }

        // let applied = self
        //     .session
        //     .batch(&batch, batch_values)
        //     .await
        //     .map_err(Error::Query)?
        //     .first_row()
        //     .map_err(Error::FirstRow)?
        //     .columns
        //     .first()
        //     .and_then(|column| column.as_ref().and_then(|column|
        // column.as_boolean()))     .ok_or(Error::MissingAppliedColumn)?;
        // if !applied {
        //     return Err(AppendStreamError::WriteConflict);
        // }

        Ok(events)
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
