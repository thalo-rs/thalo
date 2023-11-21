use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;
use thalo::stream_name::{Category, ID};
use thalo_message_store::message::GenericMessage;
use thalo_message_store::MessageStore;
use tokio::fs;
use tokio::sync::{broadcast, mpsc};
use tracing::instrument;
use wasmtime::Engine;

use crate::broadcaster::BroadcasterHandle;
use crate::command::CommandGatewayHandle;
use crate::projection::ProjectionGatewayHandle;
use crate::relay::Relay;

#[derive(Clone)]
pub struct Runtime {
    message_store: MessageStore,
    modules_path: PathBuf,
    event_tx: broadcast::Sender<GenericMessage<'static>>,
    command_gateway: CommandGatewayHandle,
    projection_gateway: ProjectionGatewayHandle,
}

impl Runtime {
    pub async fn new(
        message_store: MessageStore,
        relay: Relay,
        modules_path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.async_support(true).wasm_component_model(true);
        let engine = Engine::new(&config)?;

        let (event_tx, subscriber) = broadcast::channel(1024);
        let broadcaster = BroadcasterHandle::new(
            event_tx.clone(),
            message_store.global_event_log()?.last_position()?,
        );

        let projection_gateway = ProjectionGatewayHandle::new(message_store.clone(), subscriber);

        let modules_path = modules_path.into();
        let command_gateway = CommandGatewayHandle::new(
            engine,
            message_store.clone(),
            relay.clone(),
            broadcaster.clone(),
            modules_path.clone(),
        );

        Ok(Runtime {
            message_store,
            modules_path,
            event_tx,
            command_gateway,
            projection_gateway,
        })
    }

    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }

    #[instrument(skip(self, payload))]
    pub async fn execute(
        &self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<Vec<GenericMessage<'static>>> {
        self.command_gateway
            .execute(name, id, command, payload)
            .await
    }

    pub async fn save_module(
        &self,
        name: Category<'static>,
        module: impl AsRef<[u8]>,
    ) -> Result<()> {
        let path = self.modules_path.join(&format!("{name}.wasm"));
        fs::write(&path, module).await?;

        self.command_gateway.start_module(name, path).await
    }

    pub async fn start_projection(
        &self,
        tx: mpsc::Sender<GenericMessage<'static>>,
        name: String,
        events: Vec<String>,
    ) -> Result<()> {
        self.projection_gateway
            .start_projection(tx, name, events)
            .await
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<GenericMessage<'static>> {
        self.event_tx.subscribe()
    }

    pub async fn acknowledge_event(&self, name: impl Into<String>, global_id: u64) -> Result<()> {
        self.projection_gateway
            .acknowledge_event(name.into(), global_id)
            .await
    }
}
