use async_trait::async_trait;
use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        tls::{MakeTlsConnect, TlsConnect},
        Socket,
    },
    PostgresConnectionManager,
};

use crate::{
    Aggregate, AggregateEvent, AggregateStateMutator, Error, Event, Identity, InternalError,
    Repository,
};

/// EventStore for postgres database
#[derive(Clone, Debug)]
pub struct EventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pool: Pool<PostgresConnectionManager<Tls>>,
}

impl<Tls> EventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pub async fn connect(
        conn: &str,
        tls: Tls,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        let manager = PostgresConnectionManager::new_from_stringlike(conn, tls)?;
        let pool = Pool::builder().build(manager).await.unwrap();

        Ok(Self { pool })
    }
}

#[async_trait]
impl<Tls> Repository for EventStore<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    async fn commit<A: Aggregate>(
        &self,
        events: Vec<<A as AggregateStateMutator>::Event>,
        agg: &mut A,
    ) -> Result<(), Error>
    where
        <A as Identity>::Identity: AsRef<str>,
    {
        let agg_events: Vec<_> = events
            .iter()
            .map(|event| event.aggregate_event(Identity::identity(agg).as_ref()))
            .collect::<Result<_, _>>()?;
        self.save_events(&agg_events).await?;

        for event in events {
            <A as AggregateStateMutator>::apply(agg, event);
        }

        Ok(())
    }

    async fn save_events(&self, events: &[AggregateEvent]) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .await
            .internal_error("could not get connection from pool")?;

        let t = conn
            .build_transaction()
            .isolation_level(bb8_postgres::tokio_postgres::IsolationLevel::Serializable)
            .start()
            .await
            .internal_error("could not start transaction")?;

        for event in events {
            // Get the max sequence for the given aggregate type and id
            let row = t
                .query_one(
                    "SELECT MAX(sequence) FROM events WHERE aggregate_type = $1 AND aggregate_id = $2",
                    &[&event.aggregate_type, &event.aggregate_id],
                )
                .await
                .internal_error("could not select max sequence")?;
            let current: Option<i64> = row
                .try_get(0)
                .internal_error("could not decode max sequence as i64")?;
            let next = current.unwrap_or(-1) + 1;

            // Insert event with incremented sequence
            t.execute(
                "
                INSERT INTO events (aggregate_type, aggregate_id, sequence, event_type, event_data)
                VALUES ($1, $2, $3, $4, $5)
                ",
                &[
                    &event.aggregate_type,
                    &event.aggregate_id,
                    &next,
                    &event.event_type,
                    &event.event_data,
                ],
            )
            .await
            .internal_error("could not insert event")?;
        }
        t.commit()
            .await
            .internal_error("could not commit transaction to insert events")?;

        Ok(())
    }

    async fn load_aggregate<A: Aggregate>(
        &self,
        id: <A as Identity>::Identity,
    ) -> Result<A, Error> {
        let conn = self
            .pool
            .get()
            .await
            .internal_error("could not get connection from pool")?;

        let rows = conn
            .query(
                "
                SELECT event_data
                FROM events
                WHERE aggregate_type = $1 AND aggregate_id = $2
                ORDER BY sequence ASC
                ",
                &[&A::aggregate_type(), &id],
            )
            .await
            .internal_error("could not load events for aggregate")?;
        let events: Vec<<A as AggregateStateMutator>::Event> = rows
            .into_iter()
            .map(|row| {
                let event_data: serde_json::Value = row
                    .try_get(0)
                    .internal_error("could not decode event_data as Vec<u8>")?;

                serde_json::from_value(event_data)
                    .internal_error("could not decode event_data as aggregate event")
            })
            .collect::<Result<_, _>>()?;

        let mut aggregate = A::new_with_id(id);
        for event in events {
            aggregate.apply(event);
        }

        Ok(aggregate)
    }
}
