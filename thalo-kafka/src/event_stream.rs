use std::{fmt, sync::Arc};

use async_stream::stream;
use futures_util::stream::StreamExt;
use rdkafka::{
    consumer::{Consumer, ConsumerContext, DefaultConsumerContext, StreamConsumer},
    message::OwnedMessage,
    util::DefaultRuntime,
    Message,
};
use serde::de::DeserializeOwned;
use thalo::{
    aggregate::Aggregate,
    event::EventEnvelope,
    event_stream::{EventStream, EventStreamResult},
};

use crate::Error;

/// An event stream consuming from kafka.
///
/// Events are expected to be received with a json payload, and will be deserialized
/// with the [`thalo::aggregate::Aggregate::Event`]'s [`serde::de::DeserializeOwned`] implementation.
#[derive(Clone)]
pub struct KafkaEventStream<C = DefaultConsumerContext, R = DefaultRuntime>
where
    C: ConsumerContext,
{
    consumer: Arc<StreamConsumer<C, R>>,
    topics: Vec<String>,
}

impl<C, R> KafkaEventStream<C, R>
where
    C: ConsumerContext,
{
    /// Creates a new [`KafkaEventStream`].
    pub fn new(topics: &[impl fmt::Display], consumer: StreamConsumer<C, R>) -> Self {
        KafkaEventStream {
            consumer: Arc::new(consumer),
            topics: topics.iter().map(|topic| topic.to_string()).collect(),
        }
    }

    /// Get a reference to the stream consumer.
    pub fn consumer(&self) -> Arc<StreamConsumer<C, R>> {
        Arc::clone(&self.consumer)
    }

    /// Subscribes to topics and listens to events using the consumer, returning a stream of events.
    ///
    /// Events are expected to be JSON [`thalo::event::EventEnvelope`]s, and will be deserialized.
    ///
    /// For more documentation, see [`thalo::event_stream::EventStream`].
    pub fn listen_events<A>(
        &self,
    ) -> EventStreamResult<'_, Result<KafkaEventMessage<<A as Aggregate>::Event>, Error>, Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: 'static + Clone + fmt::Debug + DeserializeOwned + Send,
        C: 'static,
        R: Send + Sync,
    {
        let consumer = Arc::clone(&self.consumer);

        consumer
            .subscribe(&self.topics.iter().map(AsRef::as_ref).collect::<Vec<_>>())
            .map_err(Error::SubscribeTopicError)?;

        Ok((stream! {
            loop {
                yield next_event::<A, _, _>(&consumer).await;
            }
        })
        .boxed())
    }
}

/// A message received from Kafka including the event and raw Kafka message.
#[derive(Clone, Debug)]
pub struct KafkaEventMessage<E>
where
    E: Clone + fmt::Debug,
{
    /// Deserialized event.
    pub event: EventEnvelope<E>,
    /// Kafka message received.
    pub message: OwnedMessage,
}

impl<E> KafkaEventMessage<E>
where
    E: Clone + fmt::Debug,
{
    /// Map an event to a new type whilst preserving the rest of the envelope data.
    pub fn map_event<EE, F>(self, f: F) -> KafkaEventMessage<EE>
    where
        EE: Clone + fmt::Debug,
        F: FnOnce(E) -> EE,
    {
        KafkaEventMessage {
            event: self.event.map_event(f),
            message: self.message,
        }
    }
}

impl<A, C, R> EventStream<A> for KafkaEventStream<C, R>
where
    A: 'static + Aggregate,
    <A as Aggregate>::Event: Clone + fmt::Debug,
    C: 'static + ConsumerContext,
    R: Send + Sync,
{
    type Item = Result<KafkaEventMessage<<A as Aggregate>::Event>, Error>;
    type Error = Error;

    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        KafkaEventStream::listen_events::<A>(self)
    }
}

async fn next_event<A, C, R>(
    consumer: &StreamConsumer<C, R>,
) -> Result<KafkaEventMessage<<A as Aggregate>::Event>, Error>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone + fmt::Debug + DeserializeOwned,
    C: 'static + ConsumerContext,
{
    let message = consumer
        .recv()
        .await
        .map_err(Error::RecieveMessageError)?
        .detach();

    let event = match message.payload() {
        Some(payload) => match serde_json::from_slice(payload) {
            Ok(event) => event,
            Err(err) => {
                return Err(Error::MessageJsonDeserializeError {
                    message,
                    serde_err: err,
                });
            }
        },
        None => {
            return Err(Error::EmptyPayloadError(message));
        }
    };

    Ok(KafkaEventMessage { event, message })
}
