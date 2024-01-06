use std::sync::Arc;
use std::time::Duration;

use scylla::SessionBuilder;
use thalo_event_indexer::server::event_indexer_server::EventIndexerServer;
use thalo_event_indexer::{EventIndexer, ScyllaEventStore};
use tokio::sync::Mutex;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = SessionBuilder::new()
        // .known_node("127.0.0.1:9042")
        .known_node("170.64.161.102:9042")
        .build()
        .await?;
    let event_store = ScyllaEventStore::new(Arc::new(session)).await?;
    while !event_store.is_consistent().await? {
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    let event_indexer = Arc::new(Mutex::new(EventIndexer::new(event_store).await?));
    let server = thalo_event_indexer::server::Server::new(event_indexer);

    let addr = "[::1]:50051".parse().unwrap();
    println!("Server listening on {}", addr);

    Server::builder()
        .add_service(EventIndexerServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
