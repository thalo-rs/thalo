use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use redis::streams::StreamMaxlen;
use thalo_message_store::MessageStore;
use thalo_runtime::relay::{RedisRelay, Relay};
use thalo_runtime::{rpc, Runtime};
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

/// Thalo runtime - event sourcing runtime
#[derive(Parser, Debug)]
#[command(name = "thalo", version, about, long_about = None)]
struct Cli {
    /// Message store path
    #[clap(short = 's', long, default_value = "message-store.db")]
    message_store_path: PathBuf,
    /// Path to aggregate wasm modules directory
    #[clap(short = 'm', long, default_value = "modules")]
    modules_path: PathBuf,
    /// Redis relay
    #[clap(long)]
    redis: Option<String>,
    /// Redis stream max len
    #[clap(long, default_value = "10000")]
    redis_stream_max_len: usize,
    /// Template for redis streams (only `{category}` is supported)
    #[clap(long, default_value = "{category}_events")]
    redis_stream_name_template: String,
    /// Address to listen on
    #[clap(long, default_value = "[::1]:4433")]
    addr: SocketAddr,
    /// Log levels
    #[clap(
        long,
        env,
        default_value = "thalo_runtime=info,thalo_message_store=info,warn"
    )]
    log: String,
}

pub async fn start() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy(cli.log))
        .init();

    let message_store = MessageStore::open(&cli.message_store_path)?;
    let relay = match cli.redis {
        Some(params) => {
            let conn = redis::Client::open(params)?;
            Relay::Redis(RedisRelay::new(
                conn.get_multiplexed_tokio_connection().await?,
                StreamMaxlen::Approx(cli.redis_stream_max_len),
                cli.redis_stream_name_template,
            ))
        }
        None => Relay::Noop,
    };
    let runtime = Runtime::new(message_store, relay, cli.modules_path).await?;

    let command_center_server = rpc::server::CommandCenterServer::new(runtime.clone());
    let projection_server = rpc::server::ProjectionServer::new(runtime);

    Server::builder()
        .add_service(command_center_server)
        .add_service(projection_server)
        .serve(cli.addr)
        .await?;

    Ok(())
}
