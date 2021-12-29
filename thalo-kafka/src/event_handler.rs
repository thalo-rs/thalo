use std::fmt;

use async_trait::async_trait;
use futures_util::{stream::BoxStream, StreamExt};
use rdkafka::{
    consumer::{Consumer, ConsumerContext, StreamConsumer},
    message::OwnedMessage,
    Message,
};
use thalo::event::EventHandler;
use tracing::{trace, warn};

use crate::{event_stream::KafkaEventMessage, Error, KafkaClientConfig, KafkaEventStream};

/// Watch an event handler and apply incoming events.
#[async_trait]
pub trait WatchEventHandler<Event>
where
    Self: EventHandler<Event>,
    Event: Clone + fmt::Debug + Send,
    <Self as EventHandler<Event>>::Error: 'static + std::error::Error + Send,
{
    /// List of kafka topics to subscribe to.
    ///
    /// # Example
    ///
    /// ```
    /// fn topics() -> Vec<&'static str> {
    ///     vec!["user-events"]
    /// }
    /// ```
    fn topics() -> Vec<&'static str>;

    /// An event stream for receiving aggregate events.
    ///
    /// Typically, this will be an event stream from a single aggregate,
    /// but can be a merged stream to handle events from multiple aggregates.
    ///
    /// # Example
    ///
    /// ```
    /// fn event_stream(
    ///     kafka_event_stream: &mut KafkaEventStream,
    /// ) -> Result<
    ///     BoxStream<'_, Result<KafkaEventMessage<AuthEvent>, thalo_kafka::Error>>,
    ///     thalo_kafka::Error,
    /// > {
    ///     EventStream::<BankAccount>::listen_events(kafka_event_stream)
    /// }
    /// ```
    fn event_stream(
        kafka_event_stream: &mut KafkaEventStream,
    ) -> Result<BoxStream<'_, Result<KafkaEventMessage<Event>, Error>>, Error>;

    /// Watch an event handler for incoming events and handle each event with [`EventHandler::handle`].
    ///
    /// Topics should be handled by the [`WatchEventHandler::event_stream`] method.
    /// If you use [`KafkaEventStream::listen_events`] method, topics will be subscibed to automatically
    /// using the topics returned by [`WatchEventHandler::topics`] in your implementation.
    ///
    /// If your event handler returns an error, then the kafka offset will not be saved,
    /// and Kafka will re-send the event upon reconnection.
    ///
    /// # Examples
    ///
    /// Watch single event handlers.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let kafka_host = std::env::var("KAFKA_HOST").expect("missing kafka_host env var");
    ///     let database_url = std::env::var("DATABASE_URL").expect("missing database_url env var");
    ///     
    ///     let db = Database::connect(&database_url).await?;
    ///     
    ///     let projection = BankAccountProjection::new(db);
    ///     
    ///     projection.watch(&kafka_host, "bank-account").await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// Watch multiple event handlers.
    ///
    /// ```
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let kafka_host = std::env::var("KAFKA_HOST").expect("missing kafka_host env var");
    ///     let database_url = std::env::var("DATABASE_URL").expect("missing database_url env var");
    ///     
    ///     let db = Database::connect(&database_url).await?;
    ///
    ///     let projections = [
    ///         ("bank-account", BankAccountProjection::new(db.clone())),
    ///         ("transactions", TransactionsProjection::new(db)),
    ///     ];
    ///
    ///     let handles: Vec<_> = projections
    ///         .into_iter()
    ///         .map(|(group_id, projection)| {
    ///             tokio::spawn(projection.watch(&redpanda_host, group_id))
    ///         })
    ///         .collect();
    ///
    ///     for handle in handles {
    ///         handle.await?;
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    async fn watch(&self, brokers: &str, group_id: &str) -> Result<(), Error> {
        let consumer: StreamConsumer = KafkaClientConfig::new_recommended(group_id, brokers)
            .into_inner()
            .create()
            .map_err(Error::CreateStreamError)?;

        let mut kafka_event_stream = KafkaEventStream::new(&Self::topics(), consumer);
        let consumer = kafka_event_stream.consumer();

        let mut event_stream = Self::event_stream(&mut kafka_event_stream)?;
        while let Some(result) = event_stream.next().await {
            match result {
                Ok(msg) => {
                    self.handle(msg.event)
                        .await
                        .map_err(|err| Error::EventHandlerError(Box::new(err)))?;
                    trace!(
                        topic = msg.message.topic(),
                        partition = msg.message.partition(),
                        offset = msg.message.offset(),
                        "handled event"
                    );

                    if let Err(err) = consumer.store_offset(
                        msg.message.topic(),
                        msg.message.partition(),
                        msg.message.offset(),
                    ) {
                        warn!("error while storing offset: {}", err);
                    }
                }
                Err(err) => {
                    err.log();
                    err.store_offset(&consumer);
                }
            }
        }

        Ok(())
    }
}

trait StreamError: fmt::Display + Sized {
    fn get_message(&self) -> Option<&OwnedMessage>;

    fn store_offset<C, R>(&self, consumer: &StreamConsumer<C, R>)
    where
        C: ConsumerContext;

    fn log(&self) {
        if let Some(message) = self.get_message() {
            warn!(
                topic = message.topic(),
                partition = message.partition(),
                offset = message.offset(),
                "message error: {}",
                self
            );
        } else {
            warn!("message error: {}", self);
        }
    }
}

impl StreamError for Error {
    fn get_message(&self) -> Option<&OwnedMessage> {
        use Error::*;

        match &self {
            EmptyPayloadError(message) | MessageJsonDeserializeError { message, .. } => {
                Some(message)
            }
            _ => None,
        }
    }

    fn store_offset<C, R>(&self, consumer: &StreamConsumer<C, R>)
    where
        C: ConsumerContext,
    {
        if let Some(message) = self.get_message() {
            if let Err(err) =
                consumer.store_offset(message.topic(), message.partition(), message.offset())
            {
                warn!(
                    topic = message.topic(),
                    partition = message.partition(),
                    offset = message.offset(),
                    "error while storing offset: {}",
                    err
                );
            }
        }
    }
}
