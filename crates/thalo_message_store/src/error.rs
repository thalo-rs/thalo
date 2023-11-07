use thalo::EmptyStreamName;
use thiserror::Error;

/// Type alias for `Result<T, message_db::Error>`
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Represents all the ways a method can fail.
#[derive(Debug, Error)]
pub enum Error {
    /// Database error.
    #[error(transparent)]
    Database(#[from] sled::Error),

    /// Message data failed to deserialize.
    #[error("failed to deserialize data: {0}")]
    DeserializeData(serde_cbor::Error),

    /// Message metadata failed to deserialize.
    #[error("failed to deserialize metadata: {0}")]
    DeserializeMetadata(serde_json::Error),

    #[error(transparent)]
    EmptyStreamName(#[from] EmptyStreamName),

    #[error("wrong expected version: {expected_version} (Stream: {stream_name}, Stream Version: {stream_version:?})")]
    WrongExpectedVersion {
        expected_version: u64,
        stream_name: String,
        stream_version: Option<u64>,
    },
}
