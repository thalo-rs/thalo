use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use moka::future::Cache;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use serde_json::Value;
use thalo::{Category, Context, Metadata, StreamName, ID};
use thalo_message_store::{GenericMessage, MessageData, MessageStore, Stream};
use tokio::fs;
use tracing::{error, trace, warn};
use wasmtime::Engine;

use crate::module::{Event, Module, ModuleInstance};

pub struct ExecuteCommand {
    pub name: Category<'static>,
    pub id: ID<'static>,
    pub command: String,
    pub payload: Value,
    pub reply: Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
}

pub struct StartModule {
    pub name: Category<'static>,
    pub path: PathBuf,
    pub reply: Option<RpcReplyPort<()>>,
}

pub struct CommandHandler;

pub struct CommandHandlerState {
    engine: Engine,
    modules: HashMap<Category<'static>, (Module, ActorRef<ExecuteCommand>)>,
    message_store: MessageStore,
}

pub enum CommandHandlerMsg {
    Execute(ExecuteCommand),
    StartModule(StartModule),
}

#[async_trait]
impl Actor for CommandHandler {
    type State = CommandHandlerState;
    type Msg = CommandHandlerMsg;
    type Arguments = (Engine, MessageStore, PathBuf);

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        (engine, message_store, registry_path): (Engine, MessageStore, PathBuf),
    ) -> Result<Self::State, ActorProcessingErr> {
        let mut read_dir = fs::read_dir(registry_path).await?;
        let mut modules = HashMap::new();
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
            let module = Module::from_file(engine.clone(), dir_entry.path()).await?;
            let (actor, _) = Actor::spawn_linked(
                Some(module_name.clone().into_string()),
                CategoryCommandHandler,
                (message_store.clone(), module.clone()),
                myself.get_cell(),
            )
            .await?;
            modules.insert(module_name, (module, actor));
        }

        Ok(CommandHandlerState {
            engine,
            modules,
            message_store,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: CommandHandlerMsg,
        CommandHandlerState {
            engine,
            modules,
            message_store,
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            CommandHandlerMsg::Execute(exec) => {
                let Some((_, actor)) = modules.get(&exec.name).cloned() else {
                    warn!("category does not exist");
                    return Ok(());
                };

                actor.cast(exec)?;
                Ok(())
            }
            CommandHandlerMsg::StartModule(StartModule { name, path, reply }) => {
                let module = Module::from_file(engine.clone(), path).await?;
                let (actor, _) = Actor::spawn_linked(
                    Some(name.clone().into_string()),
                    CategoryCommandHandler,
                    (message_store.clone(), module.clone()),
                    myself.get_cell(),
                )
                .await?;
                modules.insert(name, (module, actor));

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
        CommandHandlerState {
            modules,
            message_store,
            ..
        }: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorPanicked(dead_actor, err) => {
                error!("{err}");
                if let Some((category, (module, module_actor))) = modules
                    .iter_mut()
                    .find(|(_, (_, actor))| dead_actor.get_id() == actor.get_cell().get_id())
                {
                    let (actor, _) = Actor::spawn_linked(
                        Some(category.clone().into_string()),
                        CategoryCommandHandler,
                        (message_store.clone(), module.clone()),
                        myself.get_cell(),
                    )
                    .await?;
                    *module_actor = actor;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub struct CategoryCommandHandler;

pub struct CategoryCommandHandlerState {
    message_store: MessageStore,
    module: Module,
    streams: Cache<
        StreamName<'static>,
        ActorRef<(
            String,
            Value,
            Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
        )>,
    >,
}

#[async_trait]
impl Actor for CategoryCommandHandler {
    type State = CategoryCommandHandlerState;
    type Msg = ExecuteCommand;
    type Arguments = (MessageStore, Module);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        (message_store, module): (MessageStore, Module),
    ) -> Result<Self::State, ActorProcessingErr> {
        let eviction_listener = move |_stream_name,
                                      actor: ActorRef<(
            String,
            Value,
            Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
        )>,
                                      _cause| {
            actor.stop(Some("cache eviction".to_string()));
        };
        let streams = Cache::builder()
            .max_capacity(100)
            .eviction_listener(eviction_listener)
            .build();

        Ok(CategoryCommandHandlerState {
            message_store,
            module,
            streams,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        ExecuteCommand {
            name,
            id,
            command,
            payload,
            reply,
        }: ExecuteCommand,
        CategoryCommandHandlerState {
            message_store,
            module,
            streams,
        }: &mut CategoryCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        let stream_name = StreamName::from_parts(name, Some(&id))?;
        let entry_res = streams
            .entry(stream_name.clone())
            .or_try_insert_with(async {
                Actor::spawn_linked(
                    None,
                    StreamCommandHandler,
                    (message_store.clone(), module.clone(), stream_name),
                    myself.get_cell(),
                )
                .await
                .map(|(actor, _)| actor)
            })
            .await;
        match entry_res {
            Ok(entry) => {
                entry.value().cast((command, payload, reply))?;
            }
            Err(err) => {
                error!("{err}");
            }
        }

        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorPanicked(cell, err) => {
                error!("stream command handler panicked: {err}");
                let stream_name = state.streams.iter().find_map(|(stream_name, actor)| {
                    if cell.get_id() == actor.get_cell().get_id() {
                        Some(stream_name)
                    } else {
                        None
                    }
                });
                if let Some(stream_name) = stream_name {
                    state.streams.remove(&stream_name).await;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub struct StreamCommandHandler;

pub struct StreamCommandHandlerState {
    stream: Stream<'static>,
    instance: ModuleInstance,
}

#[async_trait]
impl Actor for StreamCommandHandler {
    type State = StreamCommandHandlerState;
    type Msg = (
        String,
        Value,
        Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
    );
    type Arguments = (MessageStore, Module, StreamName<'static>);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        (message_store, module, stream_name): (MessageStore, Module, StreamName<'static>),
    ) -> Result<Self::State, ActorProcessingErr> {
        let mut instance = module
            .init(&stream_name.id().context("missing ID")?)
            .await?;
        let stream = message_store.stream(stream_name)?;
        for res in stream.iter_all_messages::<MessageData>() {
            let raw_message = res?;
            let message = raw_message.message()?;
            let ctx = Context {
                id: message.id,
                stream_name: message.stream_name,
                position: message.position,
                metadata: message.metadata,
                time: message.time,
            };
            let event = Event {
                event: message.msg_type,
                payload: Cow::Owned(serde_json::to_string(&message.data)?),
            };
            instance.apply(&[(ctx, event)]).await?;
            trace!(stream_name = ?stream.stream_name(), position = message.position, "applied event");
        }

        Ok(StreamCommandHandlerState { stream, instance })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        (command, payload, reply): Self::Msg,
        StreamCommandHandlerState {
            stream, instance, ..
        }: &mut StreamCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        let payload = serde_json::to_string(&payload)?;
        let events = instance.handle(&command, &payload).await?;
        if !events.is_empty() {
            let messages: Vec<_> = events
                .iter()
                .map(|event| {
                    let payload = serde_json::from_str(&event.payload)?;
                    let metadata = Metadata {
                        stream_name: stream.stream_name().clone(),
                        position: instance.sequence().map(|v| v + 1).unwrap_or(0),
                        reply_stream_name: None,
                        schema_version: None,
                        properties: HashMap::new(),
                    };

                    Ok((event.event.as_ref(), Cow::Owned(payload), metadata))
                })
                .collect::<anyhow::Result<_>>()?;

            let written_messages = stream
                .write_messages(messages.into_iter(), instance.sequence())
                .await?;

            let reply_written_messages = reply.map(|reply| {
                (
                    reply,
                    written_messages
                        .iter()
                        .map(|message| message.clone().into_owned())
                        .collect::<Vec<_>>(),
                )
            });

            let events: Vec<_> = written_messages
                .into_iter()
                .zip(events.iter())
                .map(|(message, event)| {
                    let ctx = Context {
                        id: message.id,
                        stream_name: message.stream_name,
                        position: message.position,
                        metadata: message.metadata,
                        time: message.time,
                    };
                    let event = Event {
                        event: message.msg_type,
                        payload: Cow::Borrowed(&event.payload),
                    };
                    (ctx, event)
                })
                .collect();
            instance.apply(&events).await?;

            if let Some((reply, messages)) = reply_written_messages {
                reply.send(messages)?;
            }
        } else {
            if let Some(reply) = reply {
                reply.send(vec![])?;
            }
        };

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut StreamCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        trace!("dropped stream actor");
        Ok(state.instance.resource_drop().await?)
    }
}
