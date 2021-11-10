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

/// An Aggregate represents an entity (such as a user, bank account, etc)
/// and can be rebuilt at any time from replaying previous events.
///
/// Aggregates typically handle commands and provide validation for business
/// requirements. Given a command, an aggregate can either accept or reject it.
/// If a command is accepted, typically one (sometimes many) events are returned
/// and should be saved to the event store.
///
/// Before processing a new command, an aggregate needs to replay previous events
/// or load from a snapshot.
///
/// When using an event streaming platform such as Kafka, an aggregate would
/// typically listen on it's own command stream. Eg `user-commands`, `bank-account-commands`.
/// Once a command is processed, it's result should be pushed to a `command-results` stream,
/// and events to an aggregate specific events stream (`user-events`, `bank-account-events`).
///
/// See a diagram here: https://youtu.be/b17l7LvrTco?t=1510 at 25:10
impl<T, C, E> Aggregate for T
where
    T: Identity
        + AggregateType
        + AggregateCommand<Command = C, Event = E>
        + AggregateStateMutator<Event = E>
        + Send
        + Sync,
    C: Command,
    E: Event,
{
    type Command = C;
    type Event = E;
}
