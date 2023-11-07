use serde::{Deserialize, Serialize};
use thalo::*;

export_aggregate!(Counter);

pub struct Counter {
    count: i64,
    sequence: Option<u64>,
}

#[derive(Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum Command {
    Increment { amount: i64 },
    Decrement { amount: i64 },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum Event {
    Incremented { amount: i64 },
    Decremented { amount: i64 },
}

impl Aggregate for Counter {
    type ID = String;
    type Command = Command;
    type Event = Event;
    type Error = &'static str;

    fn init(_id: Self::ID) -> Self {
        Counter {
            count: 0,
            sequence: None,
        }
    }

    fn apply(&mut self, ctx: Context, evt: Self::Event) {
        self.sequence = Some(ctx.position);

        match evt {
            Event::Incremented { amount } => self.count += amount,
            Event::Decremented { amount } => self.count -= amount,
        }
    }

    fn handle(&self, ctx: Context, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        if self.sequence.map(|seq| ctx.processed(seq)).unwrap_or(false) {
            return Ok(vec![]);
        }

        match cmd {
            Command::Increment { amount } => Ok(vec![Event::Incremented { amount }]),
            Command::Decrement { amount } => Ok(vec![Event::Decremented { amount }]),
        }
    }
}
