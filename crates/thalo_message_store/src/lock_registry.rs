use std::sync::Arc;

use dashmap::DashMap;
use thalo::Category;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct LockRegistry {
    locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
}

impl LockRegistry {
    // Initialize a new LockRegistry
    pub fn new() -> Self {
        LockRegistry {
            locks: Arc::new(DashMap::new()),
        }
    }

    // Acquire a lock for a given category
    pub fn get_category_lock(&self, category: &Category<'_>) -> Arc<Mutex<()>> {
        let category_str: &str = category.as_ref();
        match self.locks.get(category_str) {
            Some(lock) => Arc::clone(lock.value()),
            None => {
                let lock = Arc::new(Mutex::new(()));
                self.locks
                    .insert(category.clone().into_string(), Arc::clone(&lock));
                lock
            }
        }
    }
}
