use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// Database error.
    #[error(transparent)]
    DbError(#[from] bb8_postgres::tokio_postgres::Error),
    /// Deserialize database event error.
    #[error("deserialize database event {0} error: {1}")]
    DeserializeDbEvent(u64, serde_json::Error),
    /// Could not get database pool connection.
    #[error("get connection from database pool error: {0}")]
    GetDbPoolConnection(bb8_postgres::bb8::RunError<bb8_postgres::tokio_postgres::Error>),
    /// Unable to serialize event.
    #[error("serialize event error: {0}")]
    SerializeEvent(serde_json::Error),
}
