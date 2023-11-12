use async_trait::async_trait;
use moka::future::Cache;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use serde_json::Value;
use thalo::{Category, StreamName, ID};
use thalo_message_store::{GenericMessage, MessageStore};
use tracing::error;

use crate::module::Module;

use super::{
    outbox_relay::OutboxRelayRef,
    stream_command_handler::{
        StreamCommandHandler, StreamCommandHandlerArgs, StreamCommandHandlerMsg,
        StreamCommandHandlerRef,
    },
};

pub type CategoryCommandHandlerRef = ActorRef<CategoryCommandHandlerMsg>;

pub struct CategoryCommandHandler;

pub struct CategoryCommandHandlerState {
    outbox_relay: OutboxRelayRef,
    message_store: MessageStore,
    module: Module,
    streams: Cache<StreamName<'static>, StreamCommandHandlerRef>,
}

pub enum CategoryCommandHandlerMsg {
    Execute {
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
        reply: Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
    },
    UpdateOutboxActorRef {
        outbox_relay: OutboxRelayRef,
    },
}

pub struct CategoryCommandHandlerArgs {
    pub outbox_relay: OutboxRelayRef,
    pub message_store: MessageStore,
    pub module: Module,
}

#[async_trait]
impl Actor for CategoryCommandHandler {
    type State = CategoryCommandHandlerState;
    type Msg = CategoryCommandHandlerMsg;
    type Arguments = CategoryCommandHandlerArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        CategoryCommandHandlerArgs {
            outbox_relay,
            message_store,
            module,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let eviction_listener = move |_stream_name, actor: StreamCommandHandlerRef, _cause| {
            actor.stop(Some("cache eviction".to_string()));
        };
        let streams = Cache::builder()
            .max_capacity(100)
            .eviction_listener(eviction_listener)
            .build();

        Ok(CategoryCommandHandlerState {
            outbox_relay,
            message_store,
            module,
            streams,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: CategoryCommandHandlerMsg,
        CategoryCommandHandlerState {
            outbox_relay,
            message_store,
            module,
            streams,
        }: &mut CategoryCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            CategoryCommandHandlerMsg::Execute {
                name,
                id,
                command,
                payload,
                reply,
            } => {
                let stream_name = StreamName::from_parts(name, Some(&id))?;
                let entry_res = streams
                    .entry(stream_name.clone())
                    .or_try_insert_with(async {
                        Actor::spawn_linked(
                            None,
                            StreamCommandHandler,
                            StreamCommandHandlerArgs {
                                outbox_relay: outbox_relay.clone(),
                                message_store: message_store.clone(),
                                module: module.clone(),
                                stream_name,
                            },
                            myself.get_cell(),
                        )
                        .await
                        .map(|(actor, _)| actor)
                    })
                    .await;
                match entry_res {
                    Ok(entry) => {
                        entry.value().cast(StreamCommandHandlerMsg::Execute {
                            command,
                            payload,
                            reply,
                        })?;
                    }
                    Err(err) => {
                        error!("{err}");
                    }
                }
            }
            CategoryCommandHandlerMsg::UpdateOutboxActorRef {
                outbox_relay: new_outbox_relay,
            } => {
                *outbox_relay = new_outbox_relay.clone();
                for (_, stream_actor) in streams.iter() {
                    stream_actor.cast(StreamCommandHandlerMsg::UpdateOutboxActorRef {
                        outbox_relay: new_outbox_relay.clone(),
                    })?;
                }
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
