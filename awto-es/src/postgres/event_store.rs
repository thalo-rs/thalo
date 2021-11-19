use std::{
    fmt::Write,
    ops::{Bound, Range, RangeBounds},
};

use async_trait::async_trait;
use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        self,
        tls::{MakeTlsConnect, TlsConnect},
        types::ToSql,
        Socket,
    },
    PostgresConnectionManager,
};
use tracing::debug;

use crate::{
    Aggregate, AggregateEvent, AggregateEventHandler, Error, ErrorKind, Event, EventEnvelope,
    EventHandler, EventStore, Identity, InternalError, Projection,
};

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
            .internal_error("could not get connection from pool")?;

        conn.batch_execute(
            r#"
            CREATE TABLE IF NOT EXISTS "event" (
                "id"                BIGSERIAL    PRIMARY KEY,
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
        .await
        .internal_error("could not get connection from pool")?;

        Ok(())
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
        let agg_events: Vec<_> = events
            .iter()
            .map(|event| event.aggregate_event(Identity::identity(agg)))
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
            .internal_error("could not get connection from pool")?;

        let t = conn
            .build_transaction()
            .isolation_level(bb8_postgres::tokio_postgres::IsolationLevel::Serializable)
            .start()
            .await
            .internal_error("could not start transaction")?;

        let mut inserted_events = Vec::with_capacity(events.len());
        for event in events {
            // Get the max sequence for the given aggregate type and id
            let row = t
                .query_one(
                    r#"SELECT MAX("sequence") FROM "event" WHERE "aggregate_type" = $1 AND "aggregate_id" = $2"#,
                    &[&A::aggregate_type(), &event.aggregate_id],
                )
                .await
                .internal_error("could not select max sequence")?;
            let current: Option<i64> = row
                .try_get(0)
                .internal_error("could not decode max sequence as i64")?;
            let next = current.unwrap_or(-1) + 1;

            // Insert event with incremented sequence
            let event_json = serde_json::to_value(&event.event)
                .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
            let result = t.query_one(
                    r#"
                    INSERT INTO "event" ("aggregate_type", "aggregate_id", "sequence", "event_type", "event_data")
                    VALUES ($1, $2, $3, $4, $5)
                    RETURNING "id", "created_at"
                    "#,
                    &[
                        &A::aggregate_type(),
                        &event.aggregate_id,
                        &next,
                        &event.event.event_type(),
                        &event_json,
                    ],
                )
                .await
                .internal_error("could not insert event")?;
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
            .await
            .internal_error("could not insert to outbox")?;
        }
        t.commit()
            .await
            .internal_error("could not commit transaction to insert events")?;

        Ok(inserted_events)
    }

    async fn get_aggregate_events<E: Event>(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        range: Range<i64>,
    ) -> Result<Vec<EventEnvelope<E>>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .internal_error("could not get connection from pool")?;

        let mut query = r#"
            SELECT "id", "created_at", "sequence", "event_data"
            FROM "event"
            WHERE "aggregate_type" = $1 AND "aggregate_id" = $2
            "#
        .to_string();
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![&aggregate_type, &aggregate_id];
        match range.start_bound() {
            Bound::Included(from) => {
                write!(query, r#" AND "sequence" >= ${} "#, params.len() + 1).unwrap();
                params.push(from);
            }
            Bound::Excluded(from) => {
                write!(query, r#" AND "sequence" > ${} "#, params.len() + 1).unwrap();
                params.push(from);
            }
            Bound::Unbounded => {}
        }
        match range.end_bound() {
            Bound::Included(to) => {
                write!(query, r#" AND sequence <= ${} "#, params.len() + 1).unwrap();
                params.push(to);
            }
            Bound::Excluded(to) => {
                write!(query, r#" AND "sequence" < ${} "#, params.len() + 1).unwrap();
                params.push(to);
            }
            Bound::Unbounded => {}
        }
        write!(query, r#"ORDER BY "sequence" ASC"#).unwrap();

        let rows = conn
            .query(&query, &params)
            .await
            .internal_error("could not get aggregate events in range")?;

        rows.into_iter()
            .map(|row| {
                let event_json = row.get(3);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
                Ok(EventEnvelope {
                    id: row.get(0),
                    created_at: row.get(1),
                    aggregate_type: aggregate_type.to_string(),
                    aggregate_id: aggregate_id.to_string(),
                    sequence: row.get(2),
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
            .internal_error("could not get connection from pool")?;

        let mut query = r#"
            SELECT "id", "created_at", "aggregate_type", "aggregate_id", "sequence", "event_data"
            FROM "event"
            "#
        .to_string();
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
        match range.start_bound() {
            Bound::Included(from) => {
                write!(query, r#" WHERE "id" >= ${} "#, params.len() + 1).unwrap();
                params.push(from);
            }
            Bound::Excluded(from) => {
                write!(query, r#" WHERE "id" > ${} "#, params.len() + 1).unwrap();
                params.push(from);
            }
            Bound::Unbounded => {}
        }
        match range.end_bound() {
            Bound::Included(to) => {
                if matches!(range.start_bound(), Bound::Included(_) | Bound::Excluded(_)) {
                    write!(query, r#" AND "#).unwrap();
                } else {
                    write!(query, r#" WHERE "#).unwrap();
                }
                write!(query, r#" id <= ${} "#, params.len() + 1).unwrap();
                params.push(to);
            }
            Bound::Excluded(to) => {
                if matches!(range.start_bound(), Bound::Included(_) | Bound::Excluded(_)) {
                    write!(query, r#" AND "#).unwrap();
                } else {
                    write!(query, r#" WHERE "#).unwrap();
                }
                write!(query, r#" "id" < ${} "#, params.len() + 1).unwrap();
                params.push(to);
            }
            Bound::Unbounded => {}
        }
        write!(query, r#"ORDER BY "id" ASC"#).unwrap();

        let rows = conn
            .query(&query, &params)
            .await
            .internal_error("could not get events in range")?;

        rows.into_iter()
            .map(|row| {
                let event_json = row.get(5);
                let event = serde_json::from_value(event_json)
                    .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
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
            .internal_error("could not get connection from pool")?;

        let row = conn
            .query_opt(
                r#"
                SELECT "id", "created_at", "aggregate_type", "aggregate_id", "sequence", "event_data"
                FROM "event"
                WHERE "aggregate_type" = $1 AND sequence = $2
                "#,
                &[&A::aggregate_type(), &sequence],
            )
            .await
            .internal_error("could not get event from aggregate type and sequence")?;

        row.map(|row| {
            let event_json = row.get(5);
            let event = serde_json::from_value(event_json)
                .map_err(|err| Error::new(ErrorKind::DeserializeError, err))?;
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

    async fn load_aggregate<A: Aggregate>(&self, id: String) -> Result<A, Error> {
        let conn = self
            .pool
            .get()
            .await
            .internal_error("could not get connection from pool")?;

        let rows = conn
            .query(
                r#"
                SELECT "event_data"
                FROM "event"
                WHERE "aggregate_type" = $1 AND "aggregate_id" = $2
                ORDER BY "sequence" ASC
                "#,
                &[&A::aggregate_type(), &id],
            )
            .await
            .internal_error("could not load events for aggregate")?;
        let events: Vec<<A as AggregateEventHandler>::Event> = rows
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

    async fn resync_projection<P>(&self, projection: &mut P) -> Result<(), Error>
    where
        P: Projection + Send + Sync,
    {
        let mut last_event_version = projection.last_event_id().await?.unwrap_or(-1);

        loop {
            let missing_events = self
                .get_all_events::<<P as EventHandler>::Event>(
                    last_event_version + 1..last_event_version + 10,
                )
                .await?;

            if missing_events.is_empty() {
                break;
            }

            for missing_event in missing_events {
                projection
                    .handle(
                        missing_event
                            .aggregate_id
                            .parse()
                            .internal_error("cannot parse id from string")?,
                        missing_event.event.clone(),
                        missing_event.id,
                        missing_event.sequence,
                    )
                    .await?;
                last_event_version = missing_event.id;
                debug!(?missing_event, "handled missing event");
            }
        }

        Ok(())
    }
}
