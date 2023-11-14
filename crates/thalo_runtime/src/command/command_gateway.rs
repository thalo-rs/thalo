use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use serde_json::Value;
use thalo::{Category, ID};
use thalo_message_store::{GenericMessage, MessageStore};
use tokio::fs;
use tracing::{error, warn};
use wasmtime::Engine;

use crate::{command::outbox_relay::OutboxRelayArgs, module::Module, relay::Relay};

use super::{
    aggregate_command_handler::{
        AggregateCommandHandler, AggregateCommandHandlerArgs, AggregateCommandHandlerMsg,
        AggregateCommandHandlerRef,
    },
    outbox_relay::{OutboxRelay, OutboxRelayRef},
};

pub type CommandGatewayRef = ActorRef<CommandGatewayMsg>;

pub struct CommandGateway;

pub struct CommandGatewayState {
    engine: Engine,
    message_store: MessageStore,
    relay: Relay,
    modules: HashMap<Category<'static>, AggregateModuleActors>,
}

#[derive(Clone)]
struct AggregateModuleActors {
    module: Module,
    aggregate_command_handler: AggregateCommandHandlerRef,
    outbox_relay: OutboxRelayRef,
}

pub enum CommandGatewayMsg {
    Execute {
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
        reply: Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
    },
    StartModule {
        name: Category<'static>,
        path: PathBuf,
        reply: Option<RpcReplyPort<()>>,
    },
}

pub struct CommandGatewayArgs {
    pub engine: Engine,
    pub message_store: MessageStore,
    pub relay: Relay,
    pub modules_path: PathBuf,
}

#[async_trait]
impl Actor for CommandGateway {
    type State = CommandGatewayState;
    type Msg = CommandGatewayMsg;
    type Arguments = CommandGatewayArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        CommandGatewayArgs {
            engine,
            message_store,
            relay,
            modules_path,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
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
            myself.cast(CommandGatewayMsg::StartModule {
                name: module_name,
                path: dir_entry.path(),
                reply: None,
            })?;
        }

        Ok(CommandGatewayState {
            engine,
            message_store,
            relay,
            modules: HashMap::new(),
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: CommandGatewayMsg,
        CommandGatewayState {
            engine,
            message_store,
            relay,
            modules,
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            CommandGatewayMsg::Execute {
                name,
                id,
                command,
                payload,
                reply,
            } => {
                let Some(module_actors) = modules.get(&name).cloned() else {
                    warn!("aggregate does not exist");
                    if let Some(reply) = reply {
                        reply.send(vec![])?;
                    }

                    return Ok(());
                };

                module_actors.aggregate_command_handler.cast(
                    AggregateCommandHandlerMsg::Execute {
                        name,
                        id,
                        command,
                        payload,
                        reply,
                    },
                )?;

                Ok(())
            }
            CommandGatewayMsg::StartModule { name, path, reply } => {
                let aggregate_module_actors = spawn_aggregate_module_actors(
                    myself,
                    engine.clone(),
                    message_store.clone(),
                    relay.clone(),
                    name.clone(),
                    path,
                )
                .await?;
                modules.insert(name, aggregate_module_actors);

                if let Some(reply) = reply {
                    reply.send(())?;
                }

                Ok(())
            }
        }
    }

    async fn handle_supervisor_evt(
        &self,
        myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        CommandGatewayState {
            modules,
            message_store,
            relay,
            ..
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorPanicked(dead_actor, err) => {
                error!("{err}");

                for (
                    category,
                    AggregateModuleActors {
                        module,
                        aggregate_command_handler,
                        outbox_relay,
                    },
                ) in modules.iter_mut()
                {
                    let outbox_relay = if dead_actor.get_id() == outbox_relay.get_cell().get_id() {
                        // Restart outbox relay
                        let outbox = message_store.outbox(category.clone())?;
                        let (new_outbox_relay, _) = Actor::spawn_linked(
                            Some(format!("{category}_outbox_relay")),
                            OutboxRelay,
                            OutboxRelayArgs {
                                category: category.clone(),
                                outbox,
                                relay: relay.clone(),
                            },
                            myself.get_cell(),
                        )
                        .await?;

                        *outbox_relay = new_outbox_relay.clone();

                        // Send new id to aggregate command handler
                        aggregate_command_handler.cast(
                            AggregateCommandHandlerMsg::UpdateOutboxActorRef {
                                outbox_relay: new_outbox_relay.clone(),
                            },
                        )?;

                        new_outbox_relay
                    } else {
                        outbox_relay.clone()
                    };

                    if dead_actor.get_id() == aggregate_command_handler.get_cell().get_id() {
                        // Restart category command handler
                        let (new_aggregate_command_handler, _) = Actor::spawn_linked(
                            Some(category.clone().into_string()),
                            AggregateCommandHandler,
                            AggregateCommandHandlerArgs {
                                outbox_relay,
                                message_store: message_store.clone(),
                                module: module.clone(),
                            },
                            myself.get_cell(),
                        )
                        .await?;

                        *aggregate_command_handler = new_aggregate_command_handler;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

async fn spawn_aggregate_module_actors(
    myself: CommandGatewayRef,
    engine: Engine,
    message_store: MessageStore,
    relay: Relay,
    name: Category<'static>,
    path: PathBuf,
) -> Result<AggregateModuleActors, ActorProcessingErr> {
    let module = Module::from_file(engine, path).await?;
    let outbox = message_store.outbox(name.clone())?;
    let (outbox_relay, _) = Actor::spawn_linked(
        Some(format!("{name}_outbox_relay")),
        OutboxRelay,
        OutboxRelayArgs {
            category: name.clone(),
            outbox,
            relay,
        },
        myself.get_cell(),
    )
    .await?;
    let (aggregate_command_handler, _) = Actor::spawn_linked(
        Some(name.clone().into_string()),
        AggregateCommandHandler,
        AggregateCommandHandlerArgs {
            outbox_relay: outbox_relay.clone(),
            message_store,
            module: module.clone(),
        },
        myself.get_cell(),
    )
    .await?;

    Ok(AggregateModuleActors {
        module,
        aggregate_command_handler,
        outbox_relay,
    })
}
