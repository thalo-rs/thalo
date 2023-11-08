use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Result};
use ractor::rpc::CallResult;
use ractor::{Actor, ActorRef};
use serde_json::Value;
use thalo::{Category, ID};
use thalo_message_store::{GenericMessage, MessageStore};
use tokio::fs;
use tokio::task::JoinHandle;
use tracing::{instrument, warn};
use wasmtime::Engine;

use crate::command::{CommandHandler, CommandHandlerMsg, ExecuteCommand, StartModule};

#[derive(Clone)]
pub struct Runtime {
    message_store: MessageStore,
    modules_path: PathBuf,
    command_handler: ActorRef<CommandHandlerMsg>,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Runtime {
    pub async fn new(
        message_store: MessageStore,
        modules_path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.async_support(true).wasm_component_model(true);
        let engine = Engine::new(&config).unwrap();

        let modules_path = modules_path.into();
        let (command_handler, join_handle) = Actor::spawn(
            Some("command_handler".to_string()),
            CommandHandler,
            (engine, message_store.clone(), modules_path.clone()),
        )
        .await?;

        Ok(Runtime {
            message_store,
            modules_path,
            command_handler,
            join_handle: Arc::new(Mutex::new(Some(join_handle))),
        })
    }

    pub async fn wait(&self) -> Result<()> {
        let mut lock = self.join_handle.lock().unwrap();
        if let Some(join_handle) = lock.take() {
            join_handle.await?; // Handle the error as you see fit.
            Ok(())
        } else {
            Err(anyhow!(
                ".wait().await has already been called on the runtime"
            ))
        }
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
        self.command_handler
            .cast(CommandHandlerMsg::Execute(ExecuteCommand {
                name,
                id,
                command,
                payload,
                reply: None,
            }))?;

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
    ) -> Result<CallResult<Vec<GenericMessage<'static>>>> {
        let res = self
            .command_handler
            .call(
                |reply| {
                    CommandHandlerMsg::Execute(ExecuteCommand {
                        name,
                        id,
                        command,
                        payload,
                        reply: Some(reply),
                    })
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

    fn start_module(&self, name: Category<'static>, path: PathBuf) -> Result<()> {
        self.command_handler
            .cast(CommandHandlerMsg::StartModule(StartModule {
                name,
                path,
                reply: None,
            }))?;

        Ok(())
    }

    async fn start_module_wait(
        &self,
        name: Category<'static>,
        path: PathBuf,
        timeout: Option<Duration>,
    ) -> Result<CallResult<()>> {
        let res = self
            .command_handler
            .call(
                |reply| {
                    CommandHandlerMsg::StartModule(StartModule {
                        name,
                        path,
                        reply: Some(reply),
                    })
                },
                timeout,
            )
            .await?;

        Ok(res)
    }
}
