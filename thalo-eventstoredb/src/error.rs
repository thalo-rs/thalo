use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ReadStreamError(#[from] eventstore::Error),

    #[error("Wrong Expected Version Error: {0}")]
    WrongExpectedVersion(usize),

    #[error("serialize event error: {0}")]
    SerializeEvent(serde_json::Error),

    #[error("Error converting event payload to byte array: {0}")]
    EventStoreError(eventstore::Error),
}
