use std::sync::Arc;

use scylla::prepared_statement::PreparedStatement;
use scylla::transport::iterator::TypedRowIterator;
use scylla::Session;

use crate::event_processor::RecordedEvent;

pub struct EventsStream {
    session: Arc<Session>,
    select_stmt: PreparedStatement,
}

impl EventsStream {
    pub async fn new(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
    ) -> anyhow::Result<Self> {
        let select_stmt = session
            .prepare(format!(
                "
                SELECT
                    stream_name,
                    sequence,
                    global_sequence,
                    id,
                    event_type,
                    data,
                    timestamp,
                    bucket
                FROM {keyspace}.{table_name}
                WHERE bucket = ? AND global_sequence >= ?;
            "
            ))
            .await?;

        Ok(EventsStream {
            session,
            select_stmt,
        })
    }

    pub async fn fetch_events<E>(
        &self,
        bucket: u64,
        global_sequence: Option<u64>,
    ) -> anyhow::Result<TypedRowIterator<RecordedEvent<E>>> {
        let iter = self
            .session
            .execute_iter(
                self.select_stmt.clone(),
                (
                    bucket as i64,
                    global_sequence.map(|n| n as i64).unwrap_or(-1),
                ),
            )
            .await?
            .into_typed();

        Ok(iter)
    }
}
