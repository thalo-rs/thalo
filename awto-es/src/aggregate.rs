use bb8_postgres::tokio_postgres::types::ToSql;

use crate::{Command, Error, Event};

pub trait Identity {
    type Identity: ToSql + Send + Sync;

    /// Identifier
    fn identity(&self) -> &Self::Identity;

    /// Initialise with identity
    fn new_with_id(id: Self::Identity) -> Self;
}

pub trait AggregateType {
    fn aggregate_type() -> &'static str;
}

pub trait AggregateCommand {
    type Command: Command;
    type Event: Event;

    /// Public command API
    fn execute(&self, command: Self::Command) -> Result<Vec<Self::Event>, Error>;
}

pub trait AggregateStateMutator {
    type Event: Event;

    /// State mutators
    fn apply(&mut self, event: Self::Event);
}

pub trait Aggregate:
    Identity + AggregateType + AggregateCommand + AggregateStateMutator + Send + Sync
{
    type Command: Command;
    type Event: Event;
}

impl<T, E> Aggregate for T
where
    T: Identity
        + AggregateType
        + AggregateCommand<Event = E>
        + AggregateStateMutator<Event = E>
        + Send
        + Sync,
    E: Event,
{
    type Command = <T as AggregateCommand>::Command;
    type Event = E;
}
