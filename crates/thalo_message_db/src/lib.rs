use std::borrow::Cow;
use std::io::Write;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use message_db::database::MessageStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thalo::event_store::{AppendStreamError, EventData, EventStore, NewEvent, StreamEvent};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct MessageDBEventStore {
    message_store: MessageStore,
}

impl MessageDBEventStore {
    pub async fn new(url: &str) -> message_db::Result<Self> {
        Ok(MessageDBEventStore {
            message_store: MessageStore::connect(url).await?,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub global_position: i64,
    pub position: i64,
    pub time: DateTime<Utc>,
    pub stream_name: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: Option<Value>,
    pub metadata: Option<Value>,
    pub id: Uuid,
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

#[derive(Debug, Error)]
pub enum Error {
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
impl EventStore for MessageDBEventStore {
    type Event = RecordedEvent;
    type EventStream = EventStream;
    type Error = Error;

    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, AppendStreamError<Self::Error>> {
        let res = MessageStore::write_messages(&self.message_store, stream_name, &[]).await;
        match res {
            Ok(_) => {
                todo!()
            }
            Err(message_db::Error::Database(_)) => {
                todo!()
            }
        }
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
