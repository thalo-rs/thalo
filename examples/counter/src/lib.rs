use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate, Context, Error};

export_aggregate!(Counter);

#[derive(Aggregate, Serialize, Deserialize)]
#[aggregate(schema = "examples/counter/counter.esdl")]
pub struct Counter {
    id: String,
    count: i64,
    sequence: i64,
}

impl CounterAggregate for Counter {
    fn new(id: String) -> Result<Self, Error> {
        Ok(Counter {
            id,
            count: 0,
            sequence: -1,
        })
    }

    fn apply_incremented(&mut self, ctx: Context, event: Incremented) {
        self.count += event.amount as i64;
        self.sequence = ctx.position;
    }

    fn apply_decremented(&mut self, ctx: Context, event: Decremented) {
        self.count -= event.amount as i64;
        self.sequence = ctx.position;
    }

    fn handle_increment(&self, ctx: &mut Context, amount: i32) -> Result<Incremented, Error> {
        if ctx.processed(self.sequence) {
            return Err(Error::ignore());
        }

        Ok(Incremented {
            amount,
            count: self.count + amount as i64,
        })
    }

    fn handle_decrement(&self, ctx: &mut Context, amount: i32) -> Result<Decremented, Error> {
        if ctx.processed(self.sequence) {
            return Err(Error::ignore());
        }

        Ok(Decremented {
            amount,
            count: self.count - amount as i64,
        })
    }
}
