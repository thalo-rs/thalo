use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// Message had an empty payload.
    #[error("empty message payload")]
    EmptyPayloadError,
    /// Message payload was unable to be decoded to event.
    #[error("failed to decode message json: {0}")]
    MessageJsonDeserializeError(serde_json::Error),
    /// An error occured while attempting to receive a message from the stream.
    #[error("receive message error: {0}")]
    RecieveMessageError(rdkafka::error::KafkaError),
    /// An error occured while attempting to subscribe to a topic.
    #[error("subscribe topic error: {0}")]
    SubscribeTopicError(rdkafka::error::KafkaError),
}
