use crate::awto::{DomainAggregate, EventEnvelope, Store};

pub struct PostgresStore {}

impl Store for PostgresStore {
    async fn load<A>(&self, aggregate_id: &str) -> Vec<EventEnvelope<A>>
    where
        A: DomainAggregate,
    {
        "SELECT
            sequence,
            event_type,
            event_version,
            payload,
            metadata
        FROM
            events
        WHERE
            aggregate_type = $1 AND
            aggregate_id = $2
        ORDER BY
            sequence"
    }
}
