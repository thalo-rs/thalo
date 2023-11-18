use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use ractor::rpc::CallResult;
use ractor::Actor;
use serde_json::Value;
use thalo::{Category, ID};
use thalo_message_store::message::GenericMessage;
use thalo_message_store::MessageStore;
use tokio::fs;
use tokio::sync::{broadcast, mpsc};
use tracing::{instrument, warn};
use wasmtime::Engine;

use crate::broadcaster::{Broadcaster, BroadcasterArgs, BroadcasterRef};
use crate::command::{CommandGateway, CommandGatewayArgs, CommandGatewayMsg, CommandGatewayRef};
use crate::projection::{
    ProjectionGateway, ProjectionGatewayArgs, ProjectionGatewayMsg, ProjectionGatewayRef,
};
use crate::relay::Relay;

#[derive(Clone)]
pub struct Runtime {
    message_store: MessageStore,
    modules_path: PathBuf,
    broadcaster: BroadcasterRef,
    event_tx: broadcast::Sender<GenericMessage<'static>>,
    command_gateway: CommandGatewayRef,
    projection_gateway: ProjectionGatewayRef,
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
        let (broadcaster, _) = Actor::spawn(
            Some("broadcaster".to_string()),
            Broadcaster,
            BroadcasterArgs {
                tx: event_tx.clone(),
                last_position: message_store.global_event_log()?.last_position()?,
            },
        )
        .await?;

        let (projection_gateway, _) = Actor::spawn(
            Some("projection_gateway".to_string()),
            ProjectionGateway,
            ProjectionGatewayArgs {
                message_store: message_store.clone(),
                subscriber,
            },
        )
        .await?;

        let modules_path = modules_path.into();
        let (command_gateway, _) = Actor::spawn(
            Some("command_gateway".to_string()),
            CommandGateway,
            CommandGatewayArgs {
                engine,
                message_store: message_store.clone(),
                relay: relay.clone(),
                broadcaster: broadcaster.clone(),
                modules_path: modules_path.clone(),
            },
        )
        .await?;

        Ok(Runtime {
            message_store,
            modules_path,
            broadcaster,
            event_tx,
            command_gateway,
            projection_gateway,
        })
    }

    pub fn message_store(&self) -> &MessageStore {
        &self.message_store
    }

    #[instrument(skip(self, payload))]
    pub fn execute(
        &self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<()> {
        self.command_gateway.cast(CommandGatewayMsg::Execute {
            name,
            id,
            command,
            payload,
            reply: None,
        })?;

        Ok(())
    }

    #[instrument(skip(self, payload))]
    pub async fn execute_wait(
        &self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
        timeout: Option<Duration>,
    ) -> Result<CallResult<anyhow::Result<Vec<GenericMessage<'static>>>>> {
        let res = self
            .command_gateway
            .call(
                |reply| CommandGatewayMsg::Execute {
                    name,
                    id,
                    command,
                    payload,
                    reply: Some(reply),
                },
                timeout,
            )
            .await?;

        Ok(res)
    }

    pub async fn save_module(
        &self,
        name: Category<'static>,
        module: impl AsRef<[u8]>,
    ) -> Result<()> {
        let path = self.modules_path.join(&format!("{name}.wasm"));
        fs::write(&path, module).await?;

        self.start_module(name, path)
    }

    pub async fn save_module_wait(
        &self,
        name: Category<'static>,
        module: impl AsRef<[u8]>,
        timeout: Option<Duration>,
    ) -> Result<CallResult<()>> {
        let path = self.modules_path.join(&format!("{name}.wasm"));
        fs::write(&path, module).await?;

        self.start_module_wait(name, path, timeout).await
    }

    pub fn start_projection(
        &self,
        tx: mpsc::Sender<GenericMessage<'static>>,
        name: String,
        events: Vec<String>,
    ) -> Result<()> {
        self.projection_gateway
            .cast(ProjectionGatewayMsg::StartProjection { tx, name, events })?;

        Ok(())
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<GenericMessage<'static>> {
        self.event_tx.subscribe()
    }

    pub fn acknowledge_event(&self, name: impl Into<String>, global_id: u64) -> Result<()> {
        self.projection_gateway
            .cast(ProjectionGatewayMsg::AcknowledgeEvent {
                name: name.into(),
                global_id,
            })?;

        Ok(())
    }

    fn start_module(&self, name: Category<'static>, path: PathBuf) -> Result<()> {
        self.command_gateway.cast(CommandGatewayMsg::StartModule {
            name,
            path,
            reply: None,
        })?;

        Ok(())
    }

    async fn start_module_wait(
        &self,
        name: Category<'static>,
        path: PathBuf,
        timeout: Option<Duration>,
    ) -> Result<CallResult<()>> {
        let res = self
            .command_gateway
            .call(
                |reply| CommandGatewayMsg::StartModule {
                    name,
                    path,
                    reply: Some(reply),
                },
                timeout,
            )
            .await?;

        Ok(res)
    }
}
