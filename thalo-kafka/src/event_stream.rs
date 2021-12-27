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
    event::{AggregateEventEnvelope, EventHandler},
    event_stream::{EventStream, EventStreamResult},
};
use tracing::warn;

use crate::Error;

/// An event stream consuming from kafka.
///
/// Events are expected to be received with a json payload, and will be deserialized
/// with the [`thalo::aggregate::Aggregate::Event`]'s [`serde::de::DeserializeOwned`] implementation.
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

    /// Watch events for a given event handler and run the [`EventHandler::handle`](https://docs.rs/thalo/latest/thalo/event/trait.EventHandler.html) method upon incoming events.
    ///
    /// If the handle method returns an [`Result::Err`], then the kafka offset will not be stored, otherwise it will be saved.
    pub async fn watch_event_handler<A, EH>(
        &mut self,
        event_handler: &EH,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        A: 'static + Aggregate,
        <A as Aggregate>::Event: 'static + Clone + fmt::Debug + DeserializeOwned + Send,
        EH: EventHandler<<A as Aggregate>::Event>,
        <EH as EventHandler<<A as Aggregate>::Event>>::Error: 'static + std::error::Error + Send,
        C: 'static,
        R: Send + Sync,
    {
        let consumer = self.consumer();
        let mut event_stream = EventStream::<A>::listen_events(self)?;
        while let Some(result) = event_stream.next().await {
            match result {
                Ok(msg) => {
                    event_handler
                        .handle(msg.event)
                        .await
                        .map_err(|err| Error::EventHandlerError(Box::new(err)))?;

                    if let Err(err) = consumer.store_offset(
                        msg.message.topic(),
                        msg.message.partition(),
                        msg.message.offset(),
                    ) {
                        warn!("error while storing offset: {}", err);
                    }
                }
                Err(Error::EmptyPayloadError(message))
                | Err(Error::MessageJsonDeserializeError { message, .. }) => {
                    warn!(
                        topic = message.topic(),
                        partition = message.partition(),
                        offset = message.offset(),
                        "invalid message received"
                    );
                    if let Err(err) = consumer.store_offset(
                        message.topic(),
                        message.partition(),
                        message.offset(),
                    ) {
                        warn!("error while storing offset: {}", err);
                    }
                }
                Err(err) => {
                    warn!("message error: {}", err);
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct KafkaEventMessage<A>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone + fmt::Debug,
{
    pub event: AggregateEventEnvelope<A>,
    pub message: OwnedMessage,
}

impl<A, C, R> EventStream<A> for KafkaEventStream<C, R>
where
    A: 'static + Aggregate,
    <A as Aggregate>::Event: Clone + fmt::Debug,
    C: 'static + ConsumerContext,
    R: Send + Sync,
{
    type Item = Result<KafkaEventMessage<A>, Error>;
    type Error = Error;

    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        let consumer = Arc::clone(&self.consumer);

        consumer
            .subscribe(&self.topics.iter().map(AsRef::as_ref).collect::<Vec<_>>())
            .map_err(Error::SubscribeTopicError)?;

        Ok((stream! {
            loop {
                yield next_event(&consumer).await;
            }
        })
        .boxed())
    }
}

async fn next_event<A, C, R>(consumer: &StreamConsumer<C, R>) -> Result<KafkaEventMessage<A>, Error>
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
