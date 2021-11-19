#![doc = include_str!("../README.md")]
use std::env;

use bb8_postgres::{bb8::Pool, tokio_postgres::NoTls, PostgresConnectionManager};
use clap::Parser;
use outbox_relay::outbox_relay;
use tracing::{debug, info};
use tracing_subscriber::fmt::format::Format;

#[derive(Parser)]
struct Opts {
    #[clap(short, long, forbid_empty_values = true)]
    database_url: String,
    #[clap(short, long, forbid_empty_values = true)]
    redpanda_host: String,
    #[clap(short, long, default_value = "outbox", forbid_empty_values = true)]
    slot: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load cli options
    let opts: Opts = Opts::parse();

    // Initialise trace logging
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .as_deref()
                .unwrap_or("warn,outbox_relay=trace"),
        )
        .event_format(Format::default().pretty().with_source_location(false))
        .init();

    // Create database client
    debug!(database_url = %opts.database_url, "connecting to database");
    let manager = PostgresConnectionManager::new_from_stringlike(&opts.database_url, NoTls)?;
    let pool = Pool::builder().build(manager).await?;

    outbox_relay(pool, &opts.database_url, opts.redpanda_host, &opts.slot).await?;

    info!("exiting");

    Ok(())
}
