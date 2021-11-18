#![doc = include_str!("../README.md")]
use std::{process::Stdio, time::Duration};

use bb8_postgres::{
    bb8::Pool,
    tokio_postgres::{
        tls::{MakeTlsConnect, TlsConnect},
        Socket,
    },
    PostgresConnectionManager,
};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use rdkafka::{config::RDKafkaLogLevel, producer::FutureProducer, ClientConfig};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time,
};
use tracing::{error, info};

use crate::{
    change::{handle_event, Event, SlotChange},
    error::Error,
};

mod change;
pub mod error;

pub async fn outbox_relay<Tls>(
    pool: Pool<PostgresConnectionManager<Tls>>,
    database_url: &str,
    redpanda_host: impl Into<String>,
    slot: &str,
) -> Result<(), Error>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    // Create producer
    let mut producer_config = ClientConfig::new();
    producer_config
        .set("bootstrap.servers", redpanda_host)
        .set_log_level(RDKafkaLogLevel::Debug);
    let producer: FutureProducer = producer_config.create()?;

    {
        let producer = producer.clone();
        let pool = pool.clone();

        tokio::spawn(async move {
            let conn = pool
                .get()
                .await
                .expect("could not get connection from pool");
            let mut workers = FuturesUnordered::new();

            loop {
                workers.clear();
                time::sleep(Duration::from_secs(3)).await;

                let events = conn
                    .query(
                        r#"
                        SELECT
                            "event"."id",
                            "event"."created_at"::TEXT,
                            "event"."aggregate_type",
                            "event"."aggregate_id",
                            "event"."sequence",
                            "event"."event_type",
                            "event"."event_data"
                        FROM "event"
                        INNER JOIN "outbox" ON "event"."id" = "outbox"."id"
                        ORDER BY "event"."id" ASC
                        LIMIT 10
                        "#,
                        &[],
                    )
                    .await
                    .expect("select outbox events");

                let aggregate_events: Vec<Vec<Event>> = events
                    .into_iter()
                    .filter_map(|row| {
                        Some(Event {
                            id: row.get(0),
                            created_at: row.get(1),
                            aggregate_type: row.get(2),
                            aggregate_id: row.get(3),
                            sequence: row.get(4),
                            event_type: row.get(5),
                            event_data: row.get::<_, Value>(6).as_object()?.clone(),
                        })
                    })
                    .fold(Vec::new(), |mut acc, event| {
                        let inner = acc.iter_mut().find(|events| {
                            events.iter().next().unwrap().aggregate_id == event.aggregate_id
                        });
                        match inner {
                            Some(inner) => {
                                inner.push(event);
                            }
                            None => acc.push(vec![event]),
                        }
                        acc
                    });

                for events in aggregate_events {
                    let producer = producer.clone();
                    let pool = pool.clone();

                    workers.push(
                        async move {
                            for event in events {
                                let producer = producer.clone();
                                let pool = pool.clone();

                                if let Err(err) = handle_event(event, producer, pool).await {
                                    error!(?err, "error handling event");
                                }
                            }
                        }
                        .boxed(),
                    );
                }

                while workers.next().await.is_some() {}
            }
        });
    }

    Command::new("pg_recvlogical")
        .args(&[
            "-d",
            database_url,
            "--slot",
            slot,
            "--create-slot",
            "--if-not-exists",
            "-P",
            "wal2json",
        ])
        .stdout(Stdio::piped())
        .spawn()?
        .wait()
        .await?;

    let out = Command::new("pg_recvlogical")
        .args(&["-d", database_url, "--slot", slot, "--start", "-f", "-"])
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(Error::no_stdout)?;

    let mut lines = BufReader::new(out).lines();
    info!("watching changes");
    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<SlotChange>(&line) {
            Ok(SlotChange { change: changes }) => {
                for change in changes.into_iter().filter(|change| {
                    change.schema == "public"
                        && change.table == "event"
                        && change.kind == "insert"
                        && change.columnvalues.is_some()
                }) {
                    let producer = producer.clone(); // Fields are wrapped in Arc
                    let pool = pool.clone(); // Fields are wrapped in Arc
                    tokio::spawn(async move {
                        let event: Event = change.columnvalues.unwrap().try_into().unwrap();

                        if let Err(err) = handle_event(event, producer, pool).await {
                            error!(?err, "error handling event");
                        }
                    });
                }
            }
            Err(_) => {
                error!(%line, "unrecognised json format");
            }
        }
    }

    Ok(())
}
