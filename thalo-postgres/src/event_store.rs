use std::fmt::Write;

use async_trait::async_trait;
use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        tls::{MakeTlsConnect, TlsConnect},
        types::ToSql,
        IsolationLevel, Socket,
    },
    PostgresConnectionManager,
};
use serde::{de::DeserializeOwned, Serialize};
use thalo::{
    aggregate::{Aggregate, TypeId},
    event::{AggregateEventEnvelope, EventType},
    event_store::EventStore,
};

use crate::error::Error;

const INSERT_OUTBOX_EVENTS_QUERY: &str = include_str!("queries/insert_outbox_events.sql");
const LOAD_AGGREGATE_SEQUENCE_QUERY: &str = include_str!("queries/load_aggregate_sequence.sql");
const LOAD_EVENTS_QUERY: &str = include_str!("queries/load_events.sql");
const LOAD_EVENTS_BY_ID_QUERY: &str = include_str!("queries/load_events_by_id.sql");
const SAVE_EVENTS_QUERY: &str = include_str!("queries/save_events.sql");

/// Postgres event store implementation.
#[derive(Clone)]
pub struct PgEventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pool: Pool<PostgresConnectionManager<Tls>>,
}

impl<Tls> PgEventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    /// Connects to an event store.
    pub async fn connect(
        uri: impl ToString,
        tls: Tls,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        let manager = PostgresConnectionManager::new_from_stringlike(uri, tls)?;
        let pool = Pool::builder().build(manager).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl<Tls> EventStore for PgEventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Error = Error;

    async fn load_events<A>(
        &self,
        id: Option<&<A as Aggregate>::ID>,
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let conn = self.pool.get().await.map_err(Error::GetDbPoolConnection)?;

        let rows = conn
            .query(
                LOAD_EVENTS_QUERY,
                &[&<A as TypeId>::type_id(), &id.map(|id| id.to_string())],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let event_id = row.get::<_, i64>(0) as usize;

                let event_json = row.get(5);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::DeserializeDbEvent(event_id, err))?;

                Result::<_, Self::Error>::Ok(AggregateEventEnvelope::<A> {
                    id: event_id,
                    created_at: row.get(1),
                    aggregate_type: row.get(2),
                    aggregate_id: row.get(3),
                    sequence: row.get::<_, i64>(4) as usize,
                    event,
                })
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn load_events_by_id<A>(
        &self,
        ids: &[usize],
    ) -> Result<Vec<AggregateEventEnvelope<A>>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: DeserializeOwned,
    {
        let conn = self.pool.get().await.map_err(Error::GetDbPoolConnection)?;

        let rows = conn
            .query(
                LOAD_EVENTS_BY_ID_QUERY,
                &[&ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",")],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let event_id = row.get::<_, i64>(0) as usize;

                let event_json = row.get(5);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::DeserializeDbEvent(event_id, err))?;

                Result::<_, Self::Error>::Ok(AggregateEventEnvelope::<A> {
                    id: event_id,
                    created_at: row.get(1),
                    aggregate_type: row.get(2),
                    aggregate_id: row.get(3),
                    sequence: row.get::<_, i64>(4) as usize,
                    event,
                })
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<Option<usize>, Self::Error>
    where
        A: Aggregate,
    {
        let conn = self.pool.get().await.map_err(Error::GetDbPoolConnection)?;

        let row = conn
            .query_one(
                LOAD_AGGREGATE_SEQUENCE_QUERY,
                &[&<A as TypeId>::type_id(), &id.to_string()],
            )
            .await?;

        Ok(row
            .get::<_, Option<i64>>(0)
            .map(|sequence| sequence as usize))
    }

    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[<A as Aggregate>::Event],
    ) -> Result<Vec<usize>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize,
    {
        if events.is_empty() {
            return Ok(vec![]);
        }

        let sequence = self.load_aggregate_sequence::<A>(id).await?;

        let (query, values) = create_insert_events_query::<A>(id, sequence, events)?;

        let mut conn = self.pool.get().await.map_err(Error::GetDbPoolConnection)?;

        let transaction = conn
            .build_transaction()
            .isolation_level(IsolationLevel::ReadCommitted)
            .start()
            .await?;

        let rows = transaction
            .query(
                &query,
                &values
                    .iter()
                    .map(|value| value.as_ref() as &(dyn ToSql + Sync))
                    .collect::<Vec<_>>(),
            )
            .await?;

        let event_ids: Vec<_> = rows
            .into_iter()
            .map(|row| row.get::<_, i64>(0) as usize)
            .collect();
        let query = create_insert_outbox_events_query(&event_ids);

        transaction
            .execute(
                &query,
                &event_ids
                    .iter()
                    .map(|event_id| *event_id as i64)
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|event_id| event_id as &(dyn ToSql + Sync))
                    .collect::<Vec<_>>(),
            )
            .await?;

        transaction.commit().await?;

        Ok(event_ids)
    }
}

fn create_insert_events_query<A>(
    id: &<A as Aggregate>::ID,
    sequence: Option<usize>,
    events: &[<A as Aggregate>::Event],
) -> Result<(String, Vec<Box<dyn ToSql + Send + Sync>>), Error>
where
    A: Aggregate,
    <A as Aggregate>::Event: Serialize,
{
    let mut query = SAVE_EVENTS_QUERY.to_string();
    let mut values: Vec<Box<dyn ToSql + Send + Sync>> = Vec::with_capacity(events.len() * 5);
    let event_values = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            Result::<_, Error>::Ok((
                Box::new(<A as TypeId>::type_id()),
                Box::new(id.to_string()),
                Box::new(sequence.map(|sequence| sequence + index + 1).unwrap_or(0) as i64),
                Box::new(event.event_type()),
                Box::new(serde_json::to_value(event).map_err(Error::SerializeEvent)?),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let values_len = event_values.len();
    for (index, (aggregate_type, aggregate_id, sequence, event_type, event_data)) in
        event_values.into_iter().enumerate()
    {
        write!(
            query,
            "(${}, ${}, ${}, ${}, ${})",
            values.len() + 1,
            values.len() + 2,
            values.len() + 3,
            values.len() + 4,
            values.len() + 5
        )
        .unwrap();
        if index < values_len - 1 {
            write!(query, ", ").unwrap();
        }

        values.extend([
            aggregate_type,
            aggregate_id,
            sequence,
            event_type,
            event_data,
        ] as [Box<dyn ToSql + Send + Sync>; 5]);
    }
    write!(query, r#" RETURNING "id""#).unwrap();

    Ok((query, values))
}

fn create_insert_outbox_events_query(event_ids: &[usize]) -> String {
    INSERT_OUTBOX_EVENTS_QUERY.to_string()
        + &(1..event_ids.len() + 1)
            .into_iter()
            .map(|index| format!("(${})", index))
            .collect::<Vec<_>>()
            .join(", ")
}

#[cfg(test)]
mod tests {
    use thalo::tests_cfg::bank_account::{
        BankAccount, BankAccountEvent, DepositedFundsEvent, OpenedAccountEvent,
    };

    #[test]
    fn insert_events_query() -> Result<(), super::Error> {
        let id = "abc123".to_string();

        let (query, _) = super::create_insert_events_query::<BankAccount>(
            &id,
            None,
            &[
                BankAccountEvent::OpenedAccount(OpenedAccountEvent { balance: 0.0 }),
                BankAccountEvent::DepositedFunds(DepositedFundsEvent { amount: 25.0 }),
            ],
        )?;

        assert_eq!(
            query,
            r#"INSERT INTO "event" (
  "aggregate_type",
  "aggregate_id",
  "sequence",
  "event_type",
  "event_data"
) VALUES ($1, $2, $3, $4, $5), ($6, $7, $8, $9, $10) RETURNING "id""#
        );

        Ok(())
    }

    #[test]
    fn insert_outbox_events_query() {
        let query = super::create_insert_outbox_events_query(&[0, 1, 2]);

        assert_eq!(
            query,
            r#"INSERT INTO "outbox" ("id") VALUES ($1), ($2), ($3)"#
        );
    }
}
