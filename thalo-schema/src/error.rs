use thiserror::Error;

/// Error type.
#[derive(Debug, Error)]
pub enum Error {
    /// IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Serde yaml error.
    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),
}
