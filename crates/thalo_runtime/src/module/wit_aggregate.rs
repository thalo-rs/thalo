mod wit {
    wasmtime::component::bindgen!({
        path: "wit/aggregate.wit",
        world: "aggregate",
        ownership: Borrowing { duplicate_if_necessary: true },
        async: true,
    });
}

use std::borrow::Cow;

use thiserror::Error;
pub use wit::exports::aggregate::{Command, EventParam, EventResult};
pub use wit::thalo::aggregate::tracing;
pub use wit::Aggregate;

#[derive(Clone, Debug, Error)]
pub enum AggregateError {
    #[error("command {command} returned an error: {error}")]
    Command { command: String, error: String },
    #[error("failed to deserialize command {command}: {error}")]
    DeserializeCommand { command: String, error: String },
    #[error("failed to deserialize context: {0}")]
    DeserializeContext(String),
    #[error("failed to deserialize event {event}: {error}")]
    DeserializeEvent { event: String, error: String },
    #[error("failed to serialize command error {command}: {error}")]
    SerializeError { command: String, error: String },
    #[error("failed to serialize event: {0}")]
    SerializeEvent(String),
}

impl From<wit::exports::aggregate::Error> for AggregateError {
    fn from(err: wit::exports::aggregate::Error) -> Self {
        use wit::exports::aggregate::Error;

        match err {
            Error::Command((command, error)) => AggregateError::Command { command, error },
            Error::DeserializeCommand((command, error)) => {
                AggregateError::DeserializeCommand { command, error }
            }
            Error::DeserializeContext(err) => AggregateError::DeserializeContext(err),
            Error::DeserializeEvent((event, error)) => {
                AggregateError::DeserializeEvent { event, error }
            }
            Error::SerializeError((command, error)) => {
                AggregateError::SerializeError { command, error }
            }
            Error::SerializeEvent(err) => AggregateError::SerializeEvent(err),
        }
    }
}

impl TryFrom<EventResult> for super::Event<'static> {
    type Error = anyhow::Error;

    fn try_from(event: EventResult) -> Result<Self, Self::Error> {
        Ok(super::Event {
            event: Cow::Owned(event.event),
            payload: Cow::Owned(event.payload),
        })
    }
}
