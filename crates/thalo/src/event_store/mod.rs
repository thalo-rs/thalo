pub mod message;

use std::borrow::Cow;
use std::error::Error as StdError;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use self::message::Message;

#[async_trait]
pub trait EventStore: Send + Sync {
    type Event: Event;
    type EventStream: Stream<Item = Result<Self::Event, Self::Error>>;
    type Error: StdError;

    /// Append one or more events to a stream.
    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    /// Iterate events within a stream.
    async fn iter_stream(&self, stream_name: &str) -> Result<Self::EventStream, Self::Error>;
}

pub trait Event {
    /// Event type.
    fn event_type(&self) -> Cow<'_, str>;
    /// Event data.
    fn data(&self) -> Cow<'_, Value>;
    /// Sequence within the stream.
    fn sequence(&self) -> u64;
}

impl Event for Message<'_> {
    fn event_type(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.event_type)
    }

    fn data(&self) -> Cow<'_, Value> {
        Cow::Borrowed(&self.data)
    }

    fn sequence(&self) -> u64 {
        self.stream_sequence
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewEvent {
    pub event_type: String,
    pub data: Value,
}

impl NewEvent {
    pub fn new(event_type: String, data: Value) -> Self {
        NewEvent { event_type, data }
    }
}
