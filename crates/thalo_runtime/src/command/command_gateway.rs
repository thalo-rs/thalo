use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use thalo::stream_name::{Category, ID};
use thalo_message_store::{message::GenericMessage, MessageStore};
use tokio::{
    fs,
    sync::{mpsc, oneshot},
};
use tracing::{error, warn};
use wasmtime::Engine;

use crate::{broadcaster::BroadcasterHandle, module::Module, relay::Relay};

use super::{
    aggregate_command_handler::AggregateCommandHandlerHandle, outbox_relay::OutboxRelayHandle,
};

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
    ) -> Result<Vec<GenericMessage<'static>>> {
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

    pub async fn start_module(&self, name: Category<'static>, path: PathBuf) -> Result<()> {
        let (reply, recv) = oneshot::channel();
        let msg = CommandGatewayMsg::StartModule { name, path, reply };

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
        reply: oneshot::Sender<Result<Vec<GenericMessage<'static>>>>,
    },
    StartModule {
        name: Category<'static>,
        path: PathBuf,
        reply: oneshot::Sender<Result<()>>,
    },
}

async fn run_command_gateway(
    mut receiver: mpsc::Receiver<CommandGatewayMsg>,
    engine: Engine,
    message_store: MessageStore,
    relay: Relay,
    broadcaster: BroadcasterHandle,
    cache_size: u64,
    modules_path: PathBuf,
) {
    let mut cmd_gateway = CommandGateway {
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
            CommandGatewayMsg::StartModule { name, path, reply } => {
                let res = cmd_gateway.start_module(name, path).await;
                let _ = reply.send(res);
            }
        }
    }
}

struct CommandGateway {
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
    ) -> Result<Vec<GenericMessage<'static>>> {
        let Some(aggregate_command_handler) = self.modules.get(&name).cloned() else {
            return Err(anyhow!("aggregate does not exist or is not running"));
        };

        aggregate_command_handler
            .execute(name, id, command, payload)
            .await
    }

    async fn start_module(&mut self, name: Category<'static>, path: PathBuf) -> Result<()> {
        let module = Module::from_file(self.engine.clone(), path).await?;
        let outbox = self.message_store.outbox(name.clone())?;
        let outbox_relay = OutboxRelayHandle::new(name.clone(), outbox, self.relay.clone());

        let aggregate_command_handler = AggregateCommandHandlerHandle::new(
            name.clone(),
            outbox_relay.clone(),
            self.message_store.clone(),
            self.broadcaster.clone(),
            self.cache_size,
            module.clone(),
        );

        self.modules.insert(name, aggregate_command_handler);

        Ok(())
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

            let module_name = Category::new(Category::normalize(module_name))?;
            if let Err(err) = self.start_module(module_name, dir_entry.path()).await {
                error!("failed to start module: {err}");
            }
        }

        Ok(())
    }
}
