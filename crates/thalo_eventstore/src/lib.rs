mod event_stream;

use std::borrow::Cow;
use std::collections::VecDeque;

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use eventstore::{
    AppendToStreamOptions, Client, CurrentRevision, ExpectedRevision, ReadStreamOptions,
    StreamPosition,
};
use thalo::event_store::{AppendStreamError, EventData, EventStore, NewEvent, StreamEvent};

pub use crate::event_stream::EventStream;

#[derive(Clone)]
pub struct EventStoreDb {
    pub client: Client,
}

impl EventStoreDb {
    pub fn new(client: Client) -> Self {
        EventStoreDb { client }
    }

    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<RecordedEvent>, eventstore::Error> {
        let events_len = events.len();
        let events: Vec<_> = events
            .into_iter()
            .map(|event| eventstore::EventData::json(event.event_type, event.data).unwrap())
            .collect();
        let opts = AppendToStreamOptions::default().expected_revision(match expected_sequence {
            Some(n) => ExpectedRevision::Exact(n),
            None => ExpectedRevision::NoStream,
        });
        self.client
            .append_to_stream(stream_name, &opts, events)
            .await?;

        let mut stream = self
            .client
            .read_stream(
                stream_name,
                &ReadStreamOptions::default()
                    .position(StreamPosition::End)
                    .backwards()
                    .max_count(events_len),
            )
            .await?;
        let mut written_events = VecDeque::with_capacity(events_len);
        while let Some(event) = stream.next().await? {
            written_events.push_front(match event.link {
                Some(event) => RecordedEvent(event),
                None => RecordedEvent(
                    event
                        .event
                        .expect("if no link, then event should be present"),
                ),
            });
        }

        Ok(written_events.into())
    }
}

#[async_trait]
impl EventStore for EventStoreDb {
    type Event = RecordedEvent;
    type EventStream = EventStream;
    type Error = eventstore::Error;

    async fn append_to_stream(
        &self,
        stream_name: &str,
        events: Vec<NewEvent<'_>>,
        expected_sequence: Option<u64>,
    ) -> Result<Vec<Self::Event>, AppendStreamError<Self::Error>> {
        self.append_to_stream(stream_name, events, expected_sequence)
            .await
            .map_err(|err| match err {
                eventstore::Error::WrongExpectedVersion {
                    expected: _,
                    current,
                } => AppendStreamError::WrongExpectedSequence {
                    expected: expected_sequence,
                    current: match current {
                        CurrentRevision::Current(n) => Some(n),
                        CurrentRevision::NoStream => None,
                    },
                },
                err => AppendStreamError::Error(err),
            })
    }

    async fn iter_stream(
        &self,
        stream_name: &str,
        from: u64,
    ) -> Result<Self::EventStream, Self::Error> {
        let res = self
            .client
            .read_stream(
                stream_name,
                &ReadStreamOptions::default().position(match from {
                    0 => StreamPosition::Start,
                    n => StreamPosition::Position(n),
                }),
            )
            .await;
        match res {
            Ok(read_stream) => Ok(EventStream::new(Some(read_stream))),
            Err(eventstore::Error::ResourceNotFound) => Ok(EventStream::new(None)),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Deref, DerefMut)]
pub struct RecordedEvent(eventstore::RecordedEvent);

impl StreamEvent for RecordedEvent {
    fn event_type(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.event_type)
    }

    fn data(&self) -> Cow<'_, EventData> {
        Cow::Owned(self.as_json().unwrap_or_default())
    }

    fn sequence(&self) -> u64 {
        self.revision
    }
}
