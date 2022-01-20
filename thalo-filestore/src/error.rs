use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// Deserialize event error.
    #[error("deserialize event error: {0}")]
    DeserializeEvent(serde_json::Error),
    /// An IO error.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    /// Read write lock error.
    #[error("could not get read/write lock")]
    RwPoison,
    /// Unable to serialize event.
    #[error("serialize event error: {0}")]
    SerializeEvent(serde_json::Error),
}

impl From<thalo_inmemory::Error> for Error {
    fn from(err: thalo_inmemory::Error) -> Self {
        match err {
            thalo_inmemory::Error::DeserializeEvent(err) => Error::DeserializeEvent(err),
            thalo_inmemory::Error::RwPoison => Error::RwPoison,
            thalo_inmemory::Error::SerializeEvent(err) => Error::SerializeEvent(err),
        }
    }
}
