use std::error;

use awto_es::postgres::{tls::NoTls, PgEventStore};
use serde::Deserialize;
use tracing::error;
use tracing_subscriber::fmt::format::Format;

use crate::{
    command::BankAccount,
    query::{BankAccountProjector, BankAccountViewRepository},
};

mod command;
mod query;

#[derive(Deserialize, Debug)]
struct Config {
    rust_log: Option<String>,
    database_url: String,
    redpanda_host: String,
}

#[actix::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    // Load environment variables
    let cfg = envy::from_env::<Config>()?;

    // Initialise trace logging
    tracing_subscriber::fmt()
        .with_env_filter(
            cfg.rust_log
                .as_deref()
                .unwrap_or("warn,awto_es=trace,bank=trace"),
        )
        .event_format(Format::default().pretty().with_source_location(false))
        .init();

    // Create event store
    let event_store = PgEventStore::new_from_stringlike(&cfg.database_url, NoTls).await?;

    // Create repository
    let repository = BankAccountViewRepository::connect(&cfg.database_url, NoTls).await?;

    // Build and run app
    awto_es::build(event_store.clone(), &cfg.redpanda_host)
        .on_error(|err| {
            error!("{}", err);
        })
        .aggregate::<BankAccount>(500)
        .projection(BankAccountProjector::new(event_store, repository))
        .run()
        .await?;

    // Create kafka config and admin client
    // let mut consumer_config = ClientConfig::new();
    // consumer_config
    //     .set("group.id", env!("CARGO_PKG_NAME"))
    //     .set("bootstrap.servers", cfg.redpanda_host.clone())
    //     .set("allow.auto.create.topics", "true")
    //     .set("enable.auto.commit", "true")
    //     .set("api.version.request", "true")
    //     .set("broker.version.fallback", "2.4.0")
    //     .set_log_level(RDKafkaLogLevel::Debug);
    // let mut producer_config = ClientConfig::new();
    // producer_config
    //     .set("bootstrap.servers", cfg.redpanda_host)
    //     .set("api.version.request", "true")
    //     .set("broker.version.fallback", "2.4.0")
    //     .set_log_level(RDKafkaLogLevel::Debug);

    // Create event store
    // let event_store = PgEventStore::new_from_stringlike(&cfg.database_url, NoTls).await?;

    // Create workers unordered
    // let mut workers = FuturesUnordered::new();

    // Spawn command handler
    // workers.push(
    //     command_handler(
    //         event_store.clone(),
    //         consumer_config.create().expect("Consumer creation failed"),
    //         producer_config.create().expect("Consumer creation failed"),
    //     )
    //     .boxed(),
    // );

    // Spawn event handler
    // let repository = BankAccountViewRepository::connect(&cfg.database_url, NoTls)
    //     .await
    //     .expect("could not connect to bank account view repository");
    // workers.push(
    //     event_handler(
    //         event_store,
    //         repository,
    //         consumer_config.create().expect("Consumer creation failed"),
    //     )
    //     .boxed(),
    // );

    // Consume workers
    // while workers.next().await.is_some() {}

    Ok(())
}
