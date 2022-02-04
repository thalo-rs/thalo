use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Wrong expected position for eventstore write {0}")]
    WrongExpectedVersion(#[from] eventstore::WrongExpectedVersion),
    #[error("Error Reading Stream {0}")]
    ReadStreamError(eventstore::Error),
    #[error("Error Writing Stream {1} at position {0}")]
    WriteStreamError(usize, eventstore::Error),
    /// Deserialize event error.
    #[error("Deserialize event error: {0}")]
    DeserializeEvent(serde_json::Error),
    /// Unable to serialize event.
    #[error("Serialize event error: {0}")]
    SerializeEvent(serde_json::Error),
    /// Unable to serialize event payload.
    #[error("Serialize eventstore::EventData payload error: {0}")]
    SerializeEventDataPayload(serde_json::Error),
    #[error("Thalo event_store encountered EventStoreDB error {0}")]
    EventStoreDBError(#[from] eventstore::Error),
}
