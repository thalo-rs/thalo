use std::collections::HashMap;

use async_trait::async_trait;

use super::{DomainAggregate, EventEnvelope};

#[async_trait]
pub trait Store {
    /// Load all events for a particular `aggregate_id`
    async fn load<A>(&self, aggregate_id: &str) -> Vec<EventEnvelope<A>>
    where
        A: DomainAggregate;

    /// Load aggregate at current state
    async fn load_aggregate<A>(&self, aggregate_id: &str) -> A;

    /// Commit new events
    async fn commit<A>(
        &self,
        events: Vec<A::Event>,
        context: A,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<EventEnvelope<A>>, A::Error>
    where
        A: DomainAggregate;

    /// Method to wrap a set of events with the additional metadata needed for persistence and publishing
    fn wrap_events<A>(
        &self,
        aggregate_id: &str,
        current_sequence: usize,
        resultant_events: Vec<A::Event>,
        base_metadata: HashMap<String, String>,
    ) -> Vec<EventEnvelope<A>>
    where
        A: DomainAggregate,
    {
        let mut sequence = current_sequence;
        let mut wrapped_events: Vec<EventEnvelope<A>> = Vec::new();
        for payload in resultant_events {
            sequence += 1;
            let aggregate_type = A::aggregate_type().to_string();
            let aggregate_id: String = aggregate_id.to_string();
            let sequence = sequence;
            let metadata = base_metadata.clone();
            wrapped_events.push(EventEnvelope::new_with_metadata(
                aggregate_id,
                sequence,
                aggregate_type,
                payload,
                metadata,
            ));
        }
        wrapped_events
    }
}
