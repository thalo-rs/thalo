use std::error;

use super::message::{DomainCommand, DomainEvent};

pub trait DomainAggregate {
    type Event: DomainEvent;
    type Command: DomainCommand;
    type Error: error::Error;

    fn aggregate_type() -> &'static str;

    fn handle(&self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error>;

    fn apply(&mut self, event: Self::Event);
}

pub trait DomainAggregateList {
    fn exec(self);
}

impl DomainAggregateList for () {
    fn exec(self) {}
}

impl<A> DomainAggregateList for (A,)
where
    A: DomainAggregate + 'static,
{
    fn exec(self) {
        // do stuff on self.0
    }
}

impl<A, B> DomainAggregateList for (A, B)
where
    A: DomainAggregate + 'static,
    B: DomainAggregate + 'static,
{
    fn exec(self) {
        // do stuff on self.0, self.1
    }
}

impl<A, B, C> DomainAggregateList for (A, B, C)
where
    A: DomainAggregate + 'static,
    B: DomainAggregate + 'static,
    C: DomainAggregate + 'static,
{
    fn exec(self) {
        // do stuff on self.0, self.1, self.2
    }
}
