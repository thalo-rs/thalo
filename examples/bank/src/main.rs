use thalo::postgres::{tokio_postgres::tls::NoTls, PgEventStore};
use tracing_subscriber::fmt::format::Format;

use crate::command::bank_account::BankAccount;
use crate::config::Config;
use crate::query::bank_account::BankAccountProjector;

mod command;
mod config;
mod query;

#[actix::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    let cfg = envy::from_env::<Config>()?;

    // Initialise trace logging
    tracing_subscriber::fmt()
        .with_env_filter(
            cfg.rust_log
                .as_deref()
                .unwrap_or("warn,thalo=trace,bank=trace"),
        )
        .event_format(Format::default().pretty().with_source_location(false))
        .init();

    // Create event store
    let event_store = PgEventStore::new_from_stringlike(&cfg.database_url, NoTls).await?;
    event_store.create_event_table().await?;

    // Build and run app
    thalo::new(event_store.clone(), &cfg.redpanda_host)
        .on_error(|err| {
            tracing::error!("{}", err);
        })
        .aggregate::<BankAccount>(500)
        .projection(BankAccountProjector::new(event_store))
        .with_outbox_relay_from_stringlike(&cfg.database_url, NoTls, "outbox")
        .await?
        .run()
        .await?;

    Ok(())
}
