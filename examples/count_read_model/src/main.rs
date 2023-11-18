use anyhow::{anyhow, Result};
use async_trait::async_trait;
use counter::{Decremented, Incremented};
use message_db::database::{MessageStore, SubscribeToCategoryOpts};
use message_db::message::Message;
use mongodb::bson::{doc, Document};
use mongodb::options::{ClientOptions, UpdateOptions};
use mongodb::{Client, Collection};
// use thalo::consumer::{EventCollection, EventHandler, EventListener, MessageStoreEventListener};
use thalo::rpc::client::ProjectionClient;
use tracing_subscriber::EnvFilter;

#[derive(Debug, EventCollection)]
enum CounterEvent {
    Incremented(Incremented),
    Decremented(Decremented),
}

struct CounterEventHandler {
    collection: Collection<Document>,
}

#[async_trait]
impl EventHandler for CounterEventHandler {
    type Event = CounterEvent;

    async fn handle(&self, message: Message<Self::Event>) -> Result<()> {
        let id = message.stream_name.id.ok_or_else(|| anyhow!("no ID"))?;
        let cardinal_id = id.cardinal_id();
        let update = match &message.data {
            CounterEvent::Incremented(Incremented { count, .. }) => doc! { "count": count },
            CounterEvent::Decremented(Decremented { count, .. }) => doc! { "count": count },
        };

        self.collection
            .update_one(
                doc! { "id": cardinal_id },
                update,
                UpdateOptions::builder().upsert(true).build(),
            )
            .await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("thalo_client=info".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    let mut client = ProjectionClient::connect(String::from(url.clone())).await?;

    // Parse a connection string into an options struct.
    let mut client_options = ClientOptions::parse("mongodb://root:example@localhost:27017").await?;

    // Manually set an option.
    client_options.app_name = Some("Count Read Model".to_string());

    // Get a handle to the deployment.
    let client = Client::with_options(client_options)?;

    let db = client.database("read_model");
    let collection = db.collection("count");

    let message_store =
        MessageStore::connect("postgres://thalo_runtime:password@localhost:5432/postgres").await?;

    let handler = CounterEventHandler { collection };
    let listener = MessageStoreEventListener::new(message_store, handler);

    listener
        .listen(
            &SubscribeToCategoryOpts::builder()
                .identifier("count_read_model")
                .position_update_interval(1)
                .build(),
        )
        .await?;

    Ok(())
}
