use sled::transaction::ConflictableTransactionError;
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
    /// Database transaction error.
    #[error(transparent)]
    DatabaseTransaction(
        #[from] sled::transaction::TransactionError<ConflictableTransactionError<Box<Error>>>,
    ),

    /// Message data failed to deserialize.
    #[error("failed to deserialize data: {0}")]
    DeserializeData(serde_cbor::Error),

    #[error("failed to deserialize projection: {0}")]
    DeserializeProjection(bincode::Error),

    #[error("failed to serialize data: {0}")]
    SerializeData(serde_cbor::Error),

    #[error("failed to serialize projection: {0}")]
    SerializeProjection(bincode::Error),

    #[error(transparent)]
    EmptyStreamName(#[from] EmptyStreamName),

    #[error("invalid event reference: (ID: {id}, Stream Name: {stream_name})")]
    InvalidEventReference { id: u64, stream_name: String },

    #[error("invalid u64 ID")]
    InvalidU64Id,

    #[error("wrong expected version: {expected_version} (Stream: {stream_name}, Stream Version: {stream_version:?})")]
    WrongExpectedVersion {
        expected_version: u64,
        stream_name: String,
        stream_version: Option<u64>,
    },
}
