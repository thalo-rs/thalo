use serde::{Deserialize, Serialize};
use thalo::{events, export_aggregate, Aggregate, Apply, Command, Event, Handle};

export_aggregate!(Counter);

pub struct Counter {
    count: u64,
}

impl Aggregate for Counter {
    type Command = CounterCommand;
    type Event = CounterEvent;

    fn init(_id: String) -> Self {
        Counter { count: 0 }
    }
}

#[derive(Command, Deserialize)]
pub enum CounterCommand {
    Increment { amount: u64 },
}

impl Handle<CounterCommand> for Counter {
    type Error = ();

    fn handle(&self, cmd: CounterCommand) -> Result<Vec<CounterEvent>, Self::Error> {
        match cmd {
            CounterCommand::Increment { amount } => {
                events![Incremented { amount }, Incremented { amount }]
            }
        }
    }
}

#[derive(Event, Serialize, Deserialize)]
pub enum CounterEvent {
    Incremented(Incremented),
}

#[derive(Serialize, Deserialize)]
pub struct Incremented {
    pub amount: u64,
}

impl Apply<Incremented> for Counter {
    fn apply(&mut self, event: Incremented) {
        self.count += event.amount;
    }
}
