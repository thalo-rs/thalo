use async_trait::async_trait;
use moka::future::Cache;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use serde_json::Value;
use thalo::{Category, StreamName, ID};
use thalo_message_store::{message::GenericMessage, MessageStore};
use tracing::error;

use crate::{broadcaster::BroadcasterRef, module::Module};

use super::{
    entity_command_handler::{
        EntityCommandHandler, EntityCommandHandlerArgs, EntityCommandHandlerMsg,
        EntityCommandHandlerRef,
    },
    outbox_relay::OutboxRelayRef,
};

pub type AggregateCommandHandlerRef = ActorRef<AggregateCommandHandlerMsg>;

pub struct AggregateCommandHandler;

pub struct AggregateCommandHandlerState {
    outbox_relay: OutboxRelayRef,
    message_store: MessageStore,
    broadcaster: BroadcasterRef,
    module: Module,
    entity_command_handlers: Cache<StreamName<'static>, EntityCommandHandlerRef>,
}

pub enum AggregateCommandHandlerMsg {
    Execute {
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
        reply: Option<RpcReplyPort<anyhow::Result<Vec<GenericMessage<'static>>>>>,
    },
    UpdateOutboxActorRef {
        outbox_relay: OutboxRelayRef,
    },
}

pub struct AggregateCommandHandlerArgs {
    pub outbox_relay: OutboxRelayRef,
    pub message_store: MessageStore,
    pub broadcaster: BroadcasterRef,
    pub module: Module,
}

#[async_trait]
impl Actor for AggregateCommandHandler {
    type State = AggregateCommandHandlerState;
    type Msg = AggregateCommandHandlerMsg;
    type Arguments = AggregateCommandHandlerArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        AggregateCommandHandlerArgs {
            outbox_relay,
            message_store,
            broadcaster,
            module,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let eviction_listener = move |_stream_name, actor: EntityCommandHandlerRef, _cause| {
            actor.stop(Some("cache eviction".to_string()));
        };
        let entity_command_handlers = Cache::builder()
            .max_capacity(100)
            .eviction_listener(eviction_listener)
            .build();

        Ok(AggregateCommandHandlerState {
            outbox_relay,
            message_store,
            broadcaster,
            module,
            entity_command_handlers,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        msg: AggregateCommandHandlerMsg,
        AggregateCommandHandlerState {
            outbox_relay,
            message_store,
            broadcaster,
            module,
            entity_command_handlers,
        }: &mut AggregateCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            AggregateCommandHandlerMsg::Execute {
                name,
                id,
                command,
                payload,
                reply,
            } => {
                let stream_name = StreamName::from_parts(name, Some(&id))?;
                let entry_res = entity_command_handlers
                    .entry(stream_name.clone())
                    .or_try_insert_with(async {
                        Actor::spawn_linked(
                            None,
                            EntityCommandHandler,
                            EntityCommandHandlerArgs {
                                outbox_relay: outbox_relay.clone(),
                                message_store: message_store.clone(),
                                broadcaster: broadcaster.clone(),
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
                        entry.value().cast(EntityCommandHandlerMsg::Execute {
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
            AggregateCommandHandlerMsg::UpdateOutboxActorRef {
                outbox_relay: new_outbox_relay,
            } => {
                *outbox_relay = new_outbox_relay.clone();
                for (_, entity_command_handler) in entity_command_handlers.iter() {
                    entity_command_handler.cast(EntityCommandHandlerMsg::UpdateOutboxActorRef {
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
                error!("entity command handler panicked: {err}");
                let stream_name =
                    state
                        .entity_command_handlers
                        .iter()
                        .find_map(|(stream_name, actor)| {
                            if cell.get_id() == actor.get_cell().get_id() {
                                Some(stream_name)
                            } else {
                                None
                            }
                        });
                if let Some(stream_name) = stream_name {
                    state.entity_command_handlers.remove(&stream_name).await;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
