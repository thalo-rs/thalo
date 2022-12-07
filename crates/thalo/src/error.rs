use std::fmt;

use thiserror::Error;

use crate::wit_aggregate::Error as WitError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[doc(hidden)]
pub enum ErrorKind {
    #[error("command failed: {0}")]
    Command(String),
    #[error(transparent)]
    Ignore(IgnoreReason),
    #[error("failed to deserialize command: {0}")]
    DeserializeCommand(String),
    #[error("failed to deserialize event: {0}")]
    DeserializeEvent(String),
    #[error("failed to serialize event: {0}")]
    SerializeEvent(String),
    #[error("unknown command")]
    UnknownCommand,
    #[error("unknown event")]
    UnknownEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub struct IgnoreReason(Option<String>);

impl Error {
    /// Creates a new fatal error with a message.
    pub fn fatal(msg: impl Into<String>) -> Self {
        Error {
            kind: ErrorKind::Command(msg.into()),
        }
    }

    /// Ignore command.
    pub fn ignore() -> Self {
        Error {
            kind: ErrorKind::Ignore(IgnoreReason(None)),
        }
    }

    /// Ignore command with a reason.
    pub fn ignore_reason(reason: impl Into<String>) -> Self {
        Error {
            kind: ErrorKind::Ignore(IgnoreReason(Some(reason.into()))),
        }
    }
}

impl From<Error> for WitError {
    fn from(err: Error) -> Self {
        err.kind.into()
    }
}

impl From<ErrorKind> for WitError {
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::Command(msg) => WitError::Command(msg),
            ErrorKind::Ignore(reason) => WitError::Ignore(reason.0),
            ErrorKind::DeserializeCommand(msg) => WitError::DeserializeCommand(msg),
            ErrorKind::DeserializeEvent(msg) => WitError::DeserializeEvent(msg),
            ErrorKind::SerializeEvent(msg) => WitError::SerializeEvent(msg),
            ErrorKind::UnknownCommand => WitError::UnknownCommand,
            ErrorKind::UnknownEvent => WitError::UnknownEvent,
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error { kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for Error {}

impl fmt::Display for IgnoreReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(reason) => write!(f, "command ignored with reason: '{reason}'"),
            None => write!(f, "command ignored"),
        }
    }
}

impl std::error::Error for IgnoreReason {}
