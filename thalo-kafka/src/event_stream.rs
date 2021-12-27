use std::fmt;

use async_stream::try_stream;
use futures_util::stream::StreamExt;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::OwnedMessage,
    Message,
};
use serde::de::DeserializeOwned;
use thalo::{
    aggregate::Aggregate,
    event::AggregateEventEnvelope,
    event_stream::{EventStream, EventStreamResult},
};

use crate::Error;

/// An event stream consuming from kafka.
///
/// Events are expected to be received with a json payload, and will be deserialized
/// with the [`thalo::aggregate::Aggregate::Event`]'s [`serde::de::DeserializeOwned`] implementation.
pub struct KafkaEventStream {
    consumer: StreamConsumer,
    topics: Vec<String>,
}

impl KafkaEventStream {
    /// Creates a new [`KafkaEventStream`].
    pub fn new(topics: &[impl fmt::Display], consumer: StreamConsumer) -> Self {
        KafkaEventStream {
            consumer,
            topics: topics.iter().map(|topic| topic.to_string()).collect(),
        }
    }

    /// Get a reference to the stream consumer.
    pub fn consumer(&self) -> &StreamConsumer {
        &self.consumer
    }
}

pub struct KafkaEventMessage<A>
where
    A: Aggregate,
{
    pub event: AggregateEventEnvelope<A>,
    pub message: OwnedMessage,
}

impl<A: 'static + Aggregate> EventStream<A> for KafkaEventStream {
    type Item = Result<KafkaEventMessage<A>, Error>;
    type Error = Error;

    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        self.consumer
            .subscribe(&self.topics.iter().map(AsRef::as_ref).collect::<Vec<_>>())
            .map_err(Error::SubscribeTopicError)?;

        Ok((try_stream! {
            loop {
                let msg = self.consumer.recv().await.map_err(Error::RecieveMessageError)?.detach();

                let payload = msg.payload().ok_or(Error::EmptyPayloadError)?;

                let event_envelope =
                    serde_json::from_slice(payload).map_err(Error::MessageJsonDeserializeError)?;

                yield KafkaEventMessage {
                    event: event_envelope,
                    message: msg,
                };
            }
        })
        .boxed())
    }
}
