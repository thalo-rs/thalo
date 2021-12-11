use std::ops::{Bound, Range, RangeBounds};

use async_trait::async_trait;
use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        self,
        tls::{MakeTlsConnect, TlsConnect},
        Socket,
    },
    PostgresConnectionManager,
};
use sea_query::{Expr, Iden, Order, PostgresDriver, PostgresQueryBuilder, Query, Value};

use crate::{
    Aggregate, AggregateEvent, AggregateEventHandler, CombinedEvent, Error, Event, EventEnvelope,
    EventStore, Identity,
};

#[derive(Iden)]
enum EventTable {
    #[iden = "event"]
    Table,
    Id,
    CreatedAt,
    AggregateType,
    AggregateId,
    Sequence,
    EventType,
    EventData,
}

/// EventStore for postgres database
#[derive(Clone, Debug)]
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
    pub async fn new(
        conn: tokio_postgres::Config,
        tls: Tls,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        let manager = PostgresConnectionManager::new(conn, tls);
        let pool = Pool::builder().build(manager).await?;

        Ok(Self { pool })
    }

    pub async fn new_from_stringlike(
        conn: &str,
        tls: Tls,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        let manager = PostgresConnectionManager::new_from_stringlike(conn, tls)?;
        let pool = Pool::builder().build(manager).await?;

        Ok(Self { pool })
    }

    pub async fn new_from_pool(
        pool: Pool<PostgresConnectionManager<Tls>>,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        Ok(Self { pool })
    }

    pub async fn create_event_table(&self) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        conn.batch_execute(
            r#"
            CREATE TABLE IF NOT EXISTS "event" (
                "id"                BIGSERIAL    PRIMARY KEY,
                "created_at"        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
                "aggregate_type"    TEXT         NOT NULL,
                "aggregate_id"      TEXT         NOT NULL,
                "sequence"          BIGINT       NOT NULL,
                "event_type"        TEXT         NOT NULL,
                "event_data"        JSONB        NOT NULL,
                UNIQUE ("aggregate_type", "aggregate_id", "sequence")
            );

            COMMENT ON TABLE  "event"                  IS 'Events';
            COMMENT ON COLUMN "event"."id"             IS 'Auto-incrementing event id';
            COMMENT ON COLUMN "event"."aggregate_type" IS 'Aggregate type identifier';
            COMMENT ON COLUMN "event"."aggregate_id"   IS 'Aggregate instance identifier';
            COMMENT ON COLUMN "event"."sequence"       IS 'Incrementing number unique where each aggregate instance starts from 0';
            COMMENT ON COLUMN "event"."event_type"     IS 'Event type identifier, usually SCREAMING_SNAKE_CASE';
            COMMENT ON COLUMN "event"."event_data"     IS 'Event json payload';
            "#,
        )
        .await?;

        Ok(())
    }

    pub fn pool(&self) -> &Pool<PostgresConnectionManager<Tls>> {
        &self.pool
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
    async fn commit<A: Aggregate>(
        &self,
        events: Vec<<A as AggregateEventHandler>::Event>,
        agg: &mut A,
    ) -> Result<(), Error> {
        let id = Identity::identity(agg).to_string();
        let agg_events: Vec<_> = events
            .iter()
            .map(|event| event.aggregate_event(&id))
            .collect();
        self.save_events(agg_events).await?;

        for event in events {
            <A as AggregateEventHandler>::apply(agg, event);
        }

        Ok(())
    }

    async fn save_events<A: Aggregate>(
        &self,
        events: Vec<AggregateEvent<'_, A>>,
    ) -> Result<Vec<EventEnvelope<<A as Aggregate>::Event>>, Error> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let t = conn
            .build_transaction()
            .isolation_level(bb8_postgres::tokio_postgres::IsolationLevel::Serializable)
            .start()
            .await?;

        let mut inserted_events = Vec::with_capacity(events.len());
        for event in events {
            // Get the max sequence for the given aggregate type and id
            let (query, params) = Query::select()
                .expr(Expr::col(EventTable::Sequence).max())
                .from(EventTable::Table)
                .and_where(Expr::col(EventTable::AggregateType).eq(A::aggregate_type()))
                .and_where(Expr::col(EventTable::AggregateId).eq(event.aggregate_id))
                .build(PostgresQueryBuilder);
            let row = t.query_one(&query, &params.as_params()).await?;
            let current: Option<i64> = row.get(0);
            let next = current.unwrap_or(-1) + 1;

            // Insert event with incremented sequence
            let event_json =
                serde_json::to_value(&event.event).map_err(Error::SerializeEventError)?;
            let (query, params) = Query::insert()
                .into_table(EventTable::Table)
                .columns([
                    EventTable::AggregateType,
                    EventTable::AggregateId,
                    EventTable::Sequence,
                    EventTable::EventType,
                    EventTable::EventData,
                ])
                .values_panic(vec![
                    Value::from(A::aggregate_type()),
                    event.aggregate_id.into(),
                    next.into(),
                    event.event.event_type().into(),
                    event_json.into(),
                ])
                .returning(
                    Query::select()
                        .columns([EventTable::Id, EventTable::CreatedAt])
                        .take(),
                )
                .build(PostgresQueryBuilder);
            let result = t.query_one(&query, &params.as_params()).await?;

            let id = result.get(0);
            inserted_events.push(EventEnvelope {
                id,
                created_at: result.get(1),
                aggregate_type: A::aggregate_type().to_string(),
                aggregate_id: event.aggregate_id.to_string(),
                sequence: next,
                event: event.event.clone(),
            });

            t.execute(
                r#"
                INSERT INTO "outbox"
                VALUES ($1)
                "#,
                &[&id],
            )
            .await?;
        }
        t.commit().await?;

        Ok(inserted_events)
    }

    async fn get_aggregate_events<E: CombinedEvent>(
        &self,
        aggregate_types: &[&str],
        aggregate_id: Option<&str>,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let mut query = Query::select();
        query
            .columns([
                EventTable::Id,
                EventTable::CreatedAt,
                EventTable::AggregateType,
                EventTable::AggregateId,
                EventTable::Sequence,
                EventTable::EventType,
                EventTable::EventData,
            ])
            .from(EventTable::Table)
            .order_by(EventTable::Id, Order::Asc);

        if !aggregate_types.is_empty() {
            query.and_where(Expr::col(EventTable::AggregateType).is_in(aggregate_types.to_vec()));
        }

        if let Some(id) = aggregate_id.as_ref().take() {
            query.and_where(Expr::col(EventTable::AggregateId).eq(*id));
        }

        match range.start_bound() {
            Bound::Included(from) => {
                query.and_where(Expr::col(EventTable::Id).greater_or_equal(Expr::value(*from)));
            }
            Bound::Excluded(from) => {
                query.and_where(Expr::col(EventTable::Id).greater_than(Expr::value(*from)));
            }
            Bound::Unbounded => {}
        }

        match range.end_bound() {
            Bound::Included(to) => {
                query.and_where(Expr::col(EventTable::Id).less_or_equal(Expr::value(*to)));
            }
            Bound::Excluded(to) => {
                query.and_where(Expr::col(EventTable::Id).less_than(Expr::value(*to)));
            }
            Bound::Unbounded => {}
        }

        let (query, params) = query.build(PostgresQueryBuilder);
        let rows = conn.query(&query, &params.as_params()).await?;

        rows.into_iter()
            .map(|row| {
                let event_json = row.get(6);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::DeserializeDbEventError(Some(row.get(5)), err))?;
                Ok(EventEnvelope {
                    id: row.get(0),
                    created_at: row.get(1),
                    aggregate_type: row.get(2),
                    aggregate_id: row.get(3),
                    sequence: row.get(4),
                    event,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    async fn get_all_events<E: Event>(
        &self,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let mut query = Query::select();
        query
            .columns([
                EventTable::Id,
                EventTable::CreatedAt,
                EventTable::AggregateType,
                EventTable::AggregateId,
                EventTable::Sequence,
                EventTable::EventType,
                EventTable::EventData,
            ])
            .from(EventTable::Table)
            .order_by(EventTable::Id, Order::Asc);

        match range.start_bound() {
            Bound::Included(from) => {
                query.and_where(Expr::col(EventTable::Id).greater_or_equal(Expr::value(*from)));
            }
            Bound::Excluded(from) => {
                query.and_where(Expr::col(EventTable::Id).greater_than(Expr::value(*from)));
            }
            Bound::Unbounded => {}
        }

        match range.end_bound() {
            Bound::Included(to) => {
                query.and_where(Expr::col(EventTable::Id).less_or_equal(Expr::value(*to)));
            }
            Bound::Excluded(to) => {
                query.and_where(Expr::col(EventTable::Id).less_than(Expr::value(*to)));
            }
            Bound::Unbounded => {}
        }

        let (query, params) = query.build(PostgresQueryBuilder);
        let rows = conn.query(&query, &params.as_params()).await?;

        rows.into_iter()
            .map(|row| {
                let event_json = row.get(6);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::DeserializeDbEventError(row.get(5), err))?;
                Ok(EventEnvelope {
                    id: row.get(0),
                    created_at: row.get(1),
                    aggregate_type: row.get(2),
                    aggregate_id: row.get(3),
                    sequence: row.get(4),
                    event,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    async fn get_event_by_aggregate_sequence<A: Aggregate>(
        &self,
        sequence: i64,
    ) -> Result<Option<EventEnvelope<<A as Aggregate>::Event>>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let (query, params) = Query::select()
            .columns([
                EventTable::Id,
                EventTable::CreatedAt,
                EventTable::AggregateType,
                EventTable::AggregateId,
                EventTable::Sequence,
                EventTable::EventType,
                EventTable::EventData,
            ])
            .from(EventTable::Table)
            .and_where(Expr::col(EventTable::AggregateType).eq(A::aggregate_type()))
            .and_where(Expr::col(EventTable::Sequence).eq(sequence))
            .build(PostgresQueryBuilder);
        let row = conn.query_opt(&query, &params.as_params()).await?;

        row.map(|row| {
            let event_json = row.get(6);
            let event = serde_json::from_value(event_json)
                .map_err(|err| Error::DeserializeDbEventError(row.get(5), err))?;
            Ok(EventEnvelope {
                id: row.get(0),
                created_at: row.get(1),
                aggregate_type: row.get(2),
                aggregate_id: row.get(3),
                sequence: row.get(4),
                event,
            })
        })
        .transpose()
    }

    async fn load_aggregate<A: Aggregate>(&self, id: &str) -> Result<A, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(Error::GetDbPoolConnectionError)?;

        let (query, params) = Query::select()
            .columns([EventTable::EventType, EventTable::EventData])
            .from(EventTable::Table)
            .and_where(Expr::col(EventTable::AggregateType).eq(A::aggregate_type()))
            .and_where(Expr::col(EventTable::AggregateId).eq(id))
            .order_by(EventTable::Sequence, Order::Asc)
            .build(PostgresQueryBuilder);
        let rows = conn.query(&query, &params.as_params()).await?;
        let events: Vec<<A as AggregateEventHandler>::Event> = rows
            .into_iter()
            .map(|row| {
                let event_data = row.get(1);
                serde_json::from_value(event_data)
                    .map_err(|err| Error::DeserializeDbEventError(Some(row.get(0)), err))
            })
            .collect::<Result<_, _>>()?;

        let mut aggregate = A::new_with_id(id.parse().map_err(|_| Error::ParseIdentity)?);
        for event in events {
            aggregate.apply(event);
        }

        Ok(aggregate)
    }
}
