use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use thalo::stream_name::{Category, ID};
use thalo_message_store::message::Message;
use thalo_message_store::MessageStore;
use tokio::fs;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, warn};
use wasmtime::Engine;

use super::aggregate_command_handler::AggregateCommandHandlerHandle;
use super::outbox_relay::OutboxRelayHandle;
use crate::broadcaster::BroadcasterHandle;
use crate::module::Module;
use crate::relay::Relay;

#[derive(Clone)]
pub struct CommandGatewayHandle {
    sender: mpsc::Sender<CommandGatewayMsg>,
}

impl CommandGatewayHandle {
    pub fn new(
        engine: Engine,
        message_store: MessageStore,
        relay: Relay,
        broadcaster: BroadcasterHandle,
        cache_size: u64,
        modules_path: PathBuf,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        tokio::spawn(run_command_gateway(
            sender.clone(),
            receiver,
            engine,
            message_store,
            relay,
            broadcaster,
            cache_size,
            modules_path,
        ));

        CommandGatewayHandle { sender }
    }

    pub async fn execute(
        &self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<Message<'static>>, serde_json::Value>> {
        let (reply, recv) = oneshot::channel();
        let msg = CommandGatewayMsg::Execute {
            name,
            id,
            command,
            payload,
            reply,
        };

        let _ = self.sender.send(msg).await;
        recv.await.context("no response from command handler")?
    }

    pub async fn start_module_from_file(
        &self,
        name: Category<'static>,
        path: PathBuf,
    ) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = CommandGatewayMsg::StartModuleFromFile { name, path, reply };

        let _ = self.sender.send(msg).await;
        recv.await.context("no response from command gateway")?
    }

    pub async fn start_module_from_module(
        &self,
        name: Category<'static>,
        module: Module,
    ) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = CommandGatewayMsg::StartModuleFromModule {
            name,
            module,
            reply,
        };

        let _ = self.sender.send(msg).await;
        recv.await.context("no response from command gateway")?
    }
}

enum CommandGatewayMsg {
    Execute {
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
        reply: oneshot::Sender<Result<Result<Vec<Message<'static>>, serde_json::Value>>>,
    },
    StartModuleFromFile {
        name: Category<'static>,
        path: PathBuf,
        reply: oneshot::Sender<Result<()>>,
    },
    StartModuleFromModule {
        name: Category<'static>,
        module: Module,
        reply: oneshot::Sender<Result<()>>,
    },
}

async fn run_command_gateway(
    sender: mpsc::Sender<CommandGatewayMsg>,
    mut receiver: mpsc::Receiver<CommandGatewayMsg>,
    engine: Engine,
    message_store: MessageStore,
    relay: Relay,
    broadcaster: BroadcasterHandle,
    cache_size: u64,
    modules_path: PathBuf,
) {
    let mut cmd_gateway = CommandGateway {
        handle: CommandGatewayHandle { sender },
        engine,
        message_store,
        relay,
        broadcaster,
        cache_size,
        modules: HashMap::new(),
    };

    if let Err(err) = cmd_gateway.load_modules_in_dir(modules_path.clone()).await {
        error!(
            modules_path = %modules_path.display(),
            "failed to load modules in dir: {err}"
        );
    }

    while let Some(msg) = receiver.recv().await {
        match msg {
            CommandGatewayMsg::Execute {
                name,
                id,
                command,
                payload,
                reply,
            } => {
                let res = cmd_gateway.execute(name, id, command, payload).await;
                let _ = reply.send(res);
            }
            CommandGatewayMsg::StartModuleFromFile { name, path, reply } => {
                let res = cmd_gateway.start_module_from_file(name, path).await;
                let _ = reply.send(res);
            }
            CommandGatewayMsg::StartModuleFromModule {
                name,
                module,
                reply,
            } => {
                let res = cmd_gateway.start_module_from_module(name, module).await;
                let _ = reply.send(res);
            }
        }
    }

    error!("command gateway stopping");
}

struct CommandGateway {
    handle: CommandGatewayHandle,
    engine: Engine,
    message_store: MessageStore,
    relay: Relay,
    broadcaster: BroadcasterHandle,
    cache_size: u64,
    modules: HashMap<Category<'static>, AggregateCommandHandlerHandle>,
}

impl CommandGateway {
    async fn execute(
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<Message<'static>>, serde_json::Value>> {
        let Some(aggregate_command_handler) = self.modules.get(&name).cloned() else {
            return Err(anyhow!(
                "aggregate '{name}' does not exist or is not running"
            ));
        };

        aggregate_command_handler
            .execute(name, id, command, payload)
            .await
    }

    async fn start_module(&mut self, name: Category<'static>, module: Module) -> Result<()> {
        let outbox = self.message_store.outbox(name.clone())?;
        let outbox_relay = OutboxRelayHandle::new(name.clone(), outbox, self.relay.clone());

        let aggregate_command_handler = AggregateCommandHandlerHandle::new(
            self.handle.clone(),
            name.clone(),
            outbox_relay.clone(),
            self.message_store.clone(),
            self.broadcaster.clone(),
            self.cache_size,
            module,
        );

        self.modules.insert(name, aggregate_command_handler);

        Ok(())
    }

    async fn start_module_from_file(
        &mut self,
        name: Category<'static>,
        path: PathBuf,
    ) -> Result<()> {
        let module = Module::from_file(self.engine.clone(), path).await?;
        self.start_module(name, module).await
    }

    async fn start_module_from_module(
        &mut self,
        name: Category<'static>,
        module: Module,
    ) -> Result<()> {
        self.start_module(name, module).await
    }

    async fn load_modules_in_dir(&mut self, modules_path: PathBuf) -> Result<()> {
        let mut read_dir = fs::read_dir(modules_path).await?;
        while let Some(dir_entry) = read_dir.next_entry().await? {
            let Ok(file_name) = dir_entry.file_name().into_string() else {
                warn!("ignoring module with invalid file name");
                continue;
            };
            let Some((module_name, suffix)) = file_name.split_once('.') else {
                warn!("ignoring module {file_name}");
                continue;
            };
            if suffix != "wasm" {
                warn!("ignoring module {file_name} - does not end in .wasm");
                continue;
            }

            let module_name = Category::new(module_name.to_string())?;
            if let Err(err) = self
                .start_module_from_file(module_name, dir_entry.path())
                .await
            {
                error!(
                    "failed to start module '{}': {err}",
                    dir_entry.path().display()
                );
            }
        }

        Ok(())
    }
}
