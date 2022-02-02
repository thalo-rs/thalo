use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct EventRecord {
    created_at: DateTime<Utc>,
    aggregate_type: String,
    aggregate_id: String,
    sequence: usize,
    event_data: serde_json::Value,
}

#[derive(Debug, Default)]
pub struct EventStoreDBEventStore {
    /// Raw events stored in memory.
    pub events: RwLock<Vec<EventRecord>>,
}
