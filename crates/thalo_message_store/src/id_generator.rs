use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct IdGenerator {
    next_id: Arc<AtomicU64>,
}

impl IdGenerator {
    pub fn new(last_id: Option<u64>) -> Self {
        IdGenerator {
            next_id: Arc::new(AtomicU64::new(last_id.map(|id| id + 1).unwrap_or(0))),
        }
    }

    pub fn generate_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}
