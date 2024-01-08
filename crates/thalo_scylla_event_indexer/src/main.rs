use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use scylla::SessionBuilder;
use thalo_scylla_event_indexer::server::event_indexer_server::EventIndexerServer;
use thalo_scylla_event_indexer::{EventIndexer, ScyllaEventStore};
use tokio::sync::Mutex;
use tonic::transport::Server;

const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("event_indexer_descriptor");

#[derive(Parser, Debug)]
#[command(name = "thalo-scylla-event-indexer", version, about, long_about = None)]
struct Cli {
    #[clap(long, env, default_value = "0.0.0.0:50051")]
    addr: SocketAddr,
    #[clap(long, env, default_value = "127.0.0.1:9042")]
    scylla_hostname: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let session = SessionBuilder::new()
        .known_node(cli.scylla_hostname)
        .build()
        .await?;
    let event_store = ScyllaEventStore::new(Arc::new(session)).await?;
    while !event_store.is_consistent().await? {
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    let event_indexer = Arc::new(Mutex::new(EventIndexer::new(event_store).await?));
    let server = thalo_scylla_event_indexer::server::Server::new(event_indexer);

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()?;

    println!("Server listening on {}", cli.addr);

    Server::builder()
        .add_service(reflection_service)
        .add_service(EventIndexerServer::new(server))
        .serve(cli.addr)
        .await?;

    Ok(())
}
