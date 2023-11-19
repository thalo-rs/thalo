use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate, Events};

export_aggregate!(Counter);

pub struct Counter {
    count: i64,
}

impl Aggregate for Counter {
    type ID = String;
    type Command = CounterCommand;
    type Events = CounterEvents;
    type Error = &'static str;

    fn init(_id: Self::ID) -> Self {
        Counter { count: 0 }
    }

    fn apply(&mut self, evt: Event) {
        use Event::*;

        match evt {
            Incremented(IncrementedV1 { amount }) => self.count += amount,
            Decremented(DecrementedV1 { amount }) => self.count -= amount,
        }
    }

    fn handle(&self, cmd: Self::Command) -> Result<Vec<Event>, Self::Error> {
        use CounterCommand::*;
        use Event::*;

        match cmd {
            Increment { amount } => Ok(vec![Incremented(IncrementedV1 { amount })]),
            Decrement { amount } => Ok(vec![Decremented(DecrementedV1 { amount })]),
        }
    }
}

#[derive(Deserialize)]
pub enum CounterCommand {
    Increment { amount: i64 },
    Decrement { amount: i64 },
}

#[derive(Events)]
pub enum CounterEvents {
    Incremented(IncrementedV1),
    Decremented(DecrementedV1),
}

#[derive(Serialize, Deserialize)]
pub struct IncrementedV1 {
    pub amount: i64,
}

#[derive(Serialize, Deserialize)]
pub struct DecrementedV1 {
    pub amount: i64,
}
