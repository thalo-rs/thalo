use serde::{Deserialize, Serialize};
use thalo::*;

export_aggregate!(Counter);

pub struct Counter {
    count: i64,
}

impl Aggregate for Counter {
    type ID = String;
    type Command = Command;
    type Events = Events;
    type Error = &'static str;

    fn init(_id: Self::ID) -> Self {
        Counter { count: 0 }
    }

    fn apply(&mut self, evt: Event) {
        match evt {
            Event::Incremented(IncrementedV1 { amount }) => self.count += amount,
            Event::Decremented(DecrementedV1 { amount }) => self.count -= amount,
        }
    }

    fn handle(&self, cmd: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match cmd {
            Command::Increment { amount } => Ok(vec![Event::Incremented(IncrementedV1 { amount })]),
            Command::Decrement { amount } => Ok(vec![Event::Decremented(DecrementedV1 { amount })]),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum Command {
    Increment { amount: i64 },
    Decrement { amount: i64 },
}

#[derive(thalo::Events)]
pub enum Events {
    Incremented(IncrementedV1),
    Decremented(DecrementedV1),
}

#[derive(Serialize, Deserialize)]
pub struct IncrementedV1 {
    amount: i64,
}

#[derive(Serialize, Deserialize)]
pub struct DecrementedV1 {
    amount: i64,
}
