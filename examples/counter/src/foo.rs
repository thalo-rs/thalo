pub struct Counter {
    id: String,
    count: i64,
}

impl Aggregate for Counter {
    fn init(id: String) -> Counter {
        Counter { id, count: 0 }
    }
}

#[derive(Command)]
pub struct Add {
    amount: i64,
}

impl Handler<Add> for Counter {
    type Result = Added;

    fn handle(&self, msg: Add) -> Self::Result {
        Added { amount: msg.amount }
    }
}

// generated:
// pub extern "c" fn handle_add()

#[derive(Event)]
pub struct Added {
    amount: i64,
}

impl EventApplier<Added> for Counter {
    fn apply(&mut self, msg: Added) {
        self.count += msg.amount;
    }
}

// ====================

pub trait Instantiate {
    fn init(id: String) -> Self;
}

pub trait Handler<C> {
    type Result;

    fn handle(&self, cmd: C) -> Self::Result;
}

pub trait EventApplier<E> {
    fn apply(&mut self, evt: E);
}
