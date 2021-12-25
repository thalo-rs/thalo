use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// Deserialize event error.
    #[error("deserialize event error: {0}")]
    DeserializeEvent(serde_json::Error),
    /// Read write lock error.
    #[error("could not get read/write lock")]
    RwPoison,
    /// Unable to serialize event.
    #[error("serialize event error: {0}")]
    SerializeEvent(serde_json::Error),
}
