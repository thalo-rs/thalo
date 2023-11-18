use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct IdGenerator {
    counter: Arc<AtomicU64>,
}

impl IdGenerator {
    pub fn new(last_id: u64) -> Self {
        IdGenerator {
            counter: Arc::new(AtomicU64::new(last_id)),
        }
    }

    pub fn generate_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}
