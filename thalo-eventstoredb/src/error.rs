use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// Deserialize event error.
    #[error("Deserialize event error: {0}")]
    DeserializeEvent(serde_json::Error),
    /// ESDB internal error.
    #[error("EventStoreDB error: {0}")]
    EventStoreDBError(#[from] eventstore::Error),
    /// Failed to parse connection string.
    #[error("Failed to parse client settings string: {0}")]
    ParseClientSettings(#[from] eventstore::ClientSettingsParseError),
    /// Stream ID parsing error.
    #[error("Failed to parse stream ID for stream: '{0}'")]
    ParseStreamId(String),
    /// Stream ID parsing error.
    #[error("Failed to parse event metadata: {0}")]
    ParseMetadata(serde_json::Error),
    /// Error reading stream.
    #[error("Error Reading Stream: {0}")]
    ReadStreamError(eventstore::Error),
    /// Unable to serialize event.
    #[error("Serialize event error: {0}")]
    SerializeEvent(serde_json::Error),
    /// Unable to serialize event payload.
    #[error("Serialize eventstore::EventData payload error: {0}")]
    SerializeEventDataPayload(serde_json::Error),
    /// Failed to write to the stream.
    #[error("Error Writing Stream {1} at position: {0}")]
    WriteStreamError(u64, eventstore::Error),
    /// ESDB verion conflict for optimistic currency control.
    ///
    /// See https://developers.eventstore.com/clients/grpc/appending-events.html#handling-concurrency
    #[error("Wrong expected position for eventstore write: {0}")]
    WrongExpectedVersion(#[from] eventstore::WrongExpectedVersion),
}
