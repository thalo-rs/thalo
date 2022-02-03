use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ReadStreamError(#[from] eventstore::Error),

    #[error("Wrong Expected Version Error: {0}")]
    WrongExpectedVersion(usize),
}
