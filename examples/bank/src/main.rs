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
    event_store.create_event_table().await?;

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

    Ok(())
}
