use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use thalo_message_store::message_store::MessageStore;
use thalo_runtime::interface::quic::load_certs;
use thalo_runtime::interface::{self};
use thalo_runtime::runtime::Runtime;

/// Thalo runtime for event sourcing systems
#[derive(Parser, Debug)]
#[command(name = "thalo", version, about, long_about = None)]
struct Cli {
    /// file to log TLS keys to for debugging
    #[clap(long)]
    keylog: bool,
    /// TLS private key in PEM format
    #[clap(short, long, requires = "cert")]
    key: Option<PathBuf>,
    /// TLS certificate in PEM format
    #[clap(short, long, requires = "key")]
    cert: Option<PathBuf>,
    /// Enable stateless retries
    #[clap(long)]
    stateless_retry: bool,
    /// Message store path
    #[clap(short, long, default_value = "message-store.db")]
    message_store_path: PathBuf,
    /// Registry path
    #[clap(short, long, default_value = "modules")]
    modules_path: PathBuf,
    /// Address to listen on
    #[clap(long, default_value = "[::1]:4433")]
    listen: SocketAddr,
}

pub async fn start() -> Result<()> {
    let cli = Cli::parse();

    let message_store = MessageStore::open(&cli.message_store_path)?;
    let runtime = Runtime::new(message_store.clone(), cli.modules_path).await?;

    let (certs, key) = load_certs(cli.key, cli.cert).await?;
    interface::quic::run(
        certs,
        key,
        cli.keylog,
        cli.stateless_retry,
        cli.listen,
        runtime,
    )
    .await
}
