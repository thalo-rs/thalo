use async_stream::try_stream;
use futures::stream::{BoxStream, StreamExt};
use rdkafka::{consumer::StreamConsumer, Message};
use serde::de::DeserializeOwned;
use thaloto::{aggregate::Aggregate, event::AggregateEventEnvelope, event_stream::EventStream};

use crate::{Error, KafkaConfig};

/// An event stream consuming from kafka.
pub struct KafkaEventStream {
    consumer: StreamConsumer,
}

impl KafkaEventStream {
    /// Creates a new [`KafkaEventStream`].
    pub fn new(consumer: StreamConsumer) -> Self {
        KafkaEventStream { consumer }
    }

    /// Create a new [`KafkaEventStream`] from a [`KafkaConfig`].
    pub fn from_config(config: KafkaConfig) -> Self {
        todo!()
    }
}

impl<A: Aggregate> EventStream<A> for KafkaEventStream {
    type StreamOutput = Result<AggregateEventEnvelope<A>, Error>;

    fn listen_events<'a>(&'a self) -> BoxStream<'a, Self::StreamOutput>
    where
        <A as Aggregate>::Event: DeserializeOwned + Send + 'a,
    {
        (try_stream! {
            loop {
                let msg = self.consumer.recv().await.map_err(Error::RecieveMessageError)?;

                let payload = msg.payload().ok_or(Error::EmptyPayloadError)?;

                let event_envelope: AggregateEventEnvelope<A> =
                    serde_json::from_slice(payload).map_err(Error::MessageJsonDeserializeError)?;

                yield event_envelope;
            }
        })
        .boxed()
    }
}
