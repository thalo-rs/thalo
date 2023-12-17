use std::borrow::Cow;
use std::error::Error as StdError;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

pub type EventData = Map<String, Value>;

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum AppendStreamError<E> {
    #[error(transparent)]
    Error(E),
    /// Something wrote to this process at the same time.
    #[error("another process wrote to this stream at the same time causing a write conflict")]
    WriteConflict,
    /// The expected sequence did not match.
    #[error("wrong expected sequence {expected:?}, current sequence is {current:?}")]
    WrongExpectedSequence {
        expected: Option<u64>,
        current: Option<u64>,
    },
}

#[async_trait]
pub trait EventStore: Send + Sync {
    type Event: StreamEvent;
    type EventStream: Stream<Item = Result<Self::Event, Self::Error>>;
    type Error: StdError;

    /// Append one or more events to a stream.
    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, AppendStreamError<Self::Error>>;

    /// Iterate events within a stream.
    async fn iter_stream(
        &self,
        stream_name: &str,
        from: u64,
    ) -> Result<Self::EventStream, Self::Error>;
}

pub trait StreamEvent {
    /// Event type.
    fn event_type(&self) -> Cow<'_, str>;
    /// Event data.
    fn data(&self) -> Cow<'_, EventData>;
    /// Sequence within the stream.
    fn sequence(&self) -> u64;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewEvent<'a> {
    pub event_type: &'a str,
    pub data: EventData,
}

impl<'a> NewEvent<'a> {
    pub fn new(event_type: &'a str, data: EventData) -> Self {
        NewEvent { event_type, data }
    }
}
