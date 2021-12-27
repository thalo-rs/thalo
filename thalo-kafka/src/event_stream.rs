use async_stream::try_stream;
use futures_util::stream::StreamExt;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
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
pub struct KafkaEventStream {
    consumer: StreamConsumer,
    topics: Vec<String>,
}

impl KafkaEventStream {
    /// Creates a new [`KafkaEventStream`].
    pub fn new(topics: Vec<String>, consumer: StreamConsumer) -> Self {
        KafkaEventStream { consumer, topics }
    }
}

impl<A: Aggregate> EventStream<A> for KafkaEventStream {
    type Error = Error;
    type StreamError = Error;

    fn listen_events(&mut self) -> EventStreamResult<'_, A, Self::Error, Self::StreamError>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        self.consumer
            .subscribe(&self.topics.iter().map(AsRef::as_ref).collect::<Vec<_>>())
            .map_err(Error::SubscribeTopicError)?;

        Ok((try_stream! {
            loop {
                let msg = self.consumer.recv().await.map_err(Error::RecieveMessageError)?;

                let payload = msg.payload().ok_or(Error::EmptyPayloadError)?;

                let event_envelope: AggregateEventEnvelope<A> =
                    serde_json::from_slice(payload).map_err(Error::MessageJsonDeserializeError)?;

                yield event_envelope;
            }
        })
        .boxed())
    }
}
