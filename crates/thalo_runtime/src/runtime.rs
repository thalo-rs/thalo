use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;
use thalo::event_store::message::Message;
use thalo::event_store::EventStore;
use tokio::fs;
use tokio::sync::broadcast;
use tracing::instrument;
use wasmtime::Engine;

// use crate::broadcaster::BroadcasterHandle;
use crate::command::CommandGatewayHandle;
// use crate::projection::{EventInterest, ProjectionGatewayHandle};
// use crate::relay::Relay;

#[derive(Clone)]
pub struct Runtime<E: EventStore> {
    event_store: E,
    modules_path: PathBuf,
    event_tx: broadcast::Sender<Message<'static>>,
    command_gateway: CommandGatewayHandle<E>,
    // projection_gateway: ProjectionGatewayHandle,
}

impl<E> Runtime<E>
where
    E: EventStore + Clone + Send + 'static,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync,
{
    pub async fn new(
        event_store: E,
        // relay: Relay,
        modules_path: impl Into<PathBuf>,
        cache_size: u64,
    ) -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.async_support(true).wasm_component_model(true);
        let engine = Engine::new(&config)?;

        let (event_tx, subscriber) = broadcast::channel(1024);
        // let broadcaster =
        //     BroadcasterHandle::new(event_tx.clone(),
        // event_store.latest_global_version().await?);

        // let projection_gateway = ProjectionGatewayHandle::new(event_store.clone(),
        // subscriber);

        let modules_path = modules_path.into();
        let command_gateway = CommandGatewayHandle::new(
            engine,
            event_store.clone(),
            // relay.clone(),
            // broadcaster.clone(),
            cache_size,
            modules_path.clone(),
        );

        Ok(Runtime {
            event_store,
            modules_path,
            event_tx,
            command_gateway,
            // projection_gateway,
        })
    }

    pub fn event_store(&self) -> &E {
        &self.event_store
    }

    #[instrument(skip(self, payload))]
    pub async fn execute(
        &self,
        name: String,
        id: String,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<E::Event>, serde_json::Value>> {
        self.command_gateway
            .execute(name, id, command, payload)
            .await
    }

    pub async fn save_module(&self, name: String, module: impl AsRef<[u8]>) -> Result<()> {
        let path = self.modules_path.join(&format!("{name}.wasm"));
        fs::write(&path, module).await?;

        self.command_gateway
            .start_module_from_file(name, path)
            .await
    }

    // pub async fn start_projection(
    //     &self,
    //     tx: mpsc::Sender<Message<'static>>,
    //     name: String,
    //     events: Vec<EventInterest<'static>>,
    // ) -> Result<()> {
    //     self.projection_gateway
    //         .start_projection(tx, name, events)
    //         .await
    // }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Message<'static>> {
        self.event_tx.subscribe()
    }

    // pub async fn acknowledge_event(&self, name: impl Into<String>, global_id:
    // u64) -> Result<()> {     self.projection_gateway
    //         .acknowledge_event(name.into(), global_id)
    //         .await
    // }
}
