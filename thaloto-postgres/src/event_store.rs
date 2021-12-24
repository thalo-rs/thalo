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
use thaloto::{
    aggregate::Aggregate,
    event::EventType,
    event_store::{AggregateEventEnvelope, EventStore},
    TypeId,
};

use crate::error::Error;

const INSERT_OUTBOX_EVENTS_QUERY: &str = include_str!("queries/insert_outbox_events.sql");
const LOAD_AGGREGATE_SEQUENCE_QUERY: &str = include_str!("queries/load_aggregate_sequence.sql");
const LOAD_EVENTS_QUERY: &str = include_str!("queries/load_events.sql");
const SAVE_EVENTS_QUERY: &str = include_str!("queries/save_events.sql");

pub struct PgEventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pool: Pool<PostgresConnectionManager<Tls>>,
}

#[async_trait(?Send)]
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
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let rows = conn
            .query(
                LOAD_EVENTS_QUERY,
                &[&<A as TypeId>::type_id(), &id.map(|id| id.to_string())],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let event_json = row.get(6);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::DeserializeDbEvent(Some(row.get(5)), err))?;

                Result::<_, Self::Error>::Ok(AggregateEventEnvelope::<A> {
                    id: row.get(0),
                    created_at: row.get(1),
                    aggregate_type: row.get(2),
                    aggregate_id: row.get(3),
                    sequence: row.get(4),
                    event,
                })
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn load_aggregate_sequence<A>(
        &self,
        id: &<A as Aggregate>::ID,
    ) -> Result<i64, Self::Error>
    where
        A: Aggregate,
    {
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let row = conn
            .query_one(
                LOAD_AGGREGATE_SEQUENCE_QUERY,
                &[&<A as TypeId>::type_id(), &id.to_string()],
            )
            .await?;

        Ok(row.get(0))
    }

    async fn save_events<A>(
        &self,
        id: &<A as Aggregate>::ID,
        events: &[&<A as Aggregate>::Event],
    ) -> Result<Vec<i64>, Self::Error>
    where
        A: Aggregate,
        <A as Aggregate>::Event: Serialize,
    {
        let sequence = self.load_aggregate_sequence::<A>(id).await?;

        let (query, values) = create_insert_events_query::<A>(id, sequence, events)?;

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let transaction = conn
            .build_transaction()
            .isolation_level(IsolationLevel::Serializable)
            .start()
            .await?;

        let rows = transaction
            .query(
                &query,
                &values.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            )
            .await?;

        let event_ids: Vec<_> = rows.into_iter().map(|row| row.get(0)).collect();
        let query = create_insert_outbox_events_query(&event_ids);

        transaction
            .execute(
                &query,
                &values.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            )
            .await?;

        transaction.commit().await?;

        Ok(event_ids)
    }
}

fn create_insert_events_query<'a, A>(
    id: &<A as Aggregate>::ID,
    sequence: i64,
    events: &'a [&'a <A as Aggregate>::Event],
) -> Result<(String, Vec<Box<dyn ToSql + Sync>>), Error>
where
    A: Aggregate,
    <A as Aggregate>::Event: Serialize,
{
    let mut query = SAVE_EVENTS_QUERY.to_string();
    let mut values: Vec<Box<dyn ToSql + Sync>> = Vec::with_capacity(events.len() * 5);
    let event_values = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            Result::<_, Error>::Ok((
                Box::new(<A as TypeId>::type_id()),
                Box::new(id.to_string()),
                Box::new(sequence + (index as i64) + 1),
                Box::new(event.event_type()),
                Box::new(serde_json::to_string(event).map_err(Error::SerializeEventError)?),
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
        ] as [Box<dyn ToSql + Sync>; 5]);
    }
    write!(query, r#" RETURNING "id""#).unwrap();

    Ok((query, values))
}

fn create_insert_outbox_events_query(event_ids: &[i64]) -> String {
    INSERT_OUTBOX_EVENTS_QUERY.to_string()
        + &(1..event_ids.len() + 1)
            .into_iter()
            .map(|index| format!("(${})", index))
            .collect::<Vec<_>>()
            .join(", ")
}

#[cfg(test)]
mod test {
    use thaloto::tests_cfg::bank_account::{BankAccount, BankAccountEvent};

    #[test]
    fn insert_events_query() -> Result<(), super::Error> {
        let id = "abc123".to_string();

        let (query, _) = super::create_insert_events_query::<BankAccount>(
            &id,
            0,
            &[
                &BankAccountEvent::OpenedAccount { balance: 0.0 },
                &BankAccountEvent::DepositedFunds { amount: 25.0 },
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
