use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to create stream.
    #[error("failed to create stream")]
    CreateStreamError(rdkafka::error::KafkaError),
    /// Message had an empty payload.
    #[error("empty message payload")]
    EmptyPayloadError(rdkafka::message::OwnedMessage),
    /// Event handler error.
    #[error("event handler error: {0}")]
    EventHandlerError(Box<dyn 'static + std::error::Error + Send>),
    /// Message payload was unable to be decoded to event.
    #[error("failed to decode message json: {serde_err}")]
    MessageJsonDeserializeError {
        /// Received kafka message.
        message: rdkafka::message::OwnedMessage,
        /// Serde json error.
        serde_err: serde_json::Error,
    },
    /// An error occured while attempting to receive a message from the stream.
    #[error("receive message error: {0}")]
    RecieveMessageError(rdkafka::error::KafkaError),
    /// An error occured while attempting to subscribe to a topic.
    #[error("subscribe topic error: {0}")]
    SubscribeTopicError(rdkafka::error::KafkaError),
}
