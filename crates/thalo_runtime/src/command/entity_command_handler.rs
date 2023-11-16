use std::borrow::Cow;

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde_json::Value;
use thalo::StreamName;
use thalo_message_store::{GenericMessage, MessageData, MessageStore, Stream};
use tracing::{error, trace};

use crate::module::{Event, Module, ModuleInstance};

use super::outbox_relay::{OutboxRelayMsg, OutboxRelayRef};

pub type EntityCommandHandlerRef = ActorRef<EntityCommandHandlerMsg>;

pub struct EntityCommandHandler;

pub struct EntityCommandHandlerState {
    outbox_relay: OutboxRelayRef,
    stream: Stream<'static>,
    instance: ModuleInstance,
}

pub enum EntityCommandHandlerMsg {
    Execute {
        command: String,
        payload: Value,
        reply: Option<RpcReplyPort<Vec<GenericMessage<'static>>>>,
    },
    UpdateOutboxActorRef {
        outbox_relay: OutboxRelayRef,
    },
}

pub struct EntityCommandHandlerArgs {
    pub outbox_relay: OutboxRelayRef,
    pub message_store: MessageStore,
    pub module: Module,
    pub stream_name: StreamName<'static>,
}

#[async_trait]
impl Actor for EntityCommandHandler {
    type State = EntityCommandHandlerState;
    type Msg = EntityCommandHandlerMsg;
    type Arguments = EntityCommandHandlerArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        EntityCommandHandlerArgs {
            outbox_relay,
            message_store,
            module,
            stream_name,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let mut instance = module
            .init(&stream_name.id().context("missing ID")?)
            .await?;
        let stream = message_store.stream(stream_name)?;
        for res in stream.iter_all_messages::<MessageData>() {
            let raw_message = res?;
            let message = raw_message.message()?;
            let event = Event {
                event: message.msg_type,
                payload: Cow::Owned(serde_json::to_string(&message.data)?),
            };
            instance.apply(&[(message.position, event)]).await?;
            trace!(stream_name = ?stream.stream_name(), position = message.position, "applied event");
        }

        Ok(EntityCommandHandlerState {
            stream,
            instance,
            outbox_relay,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        EntityCommandHandlerState {
            outbox_relay,
            stream,
            instance,
        }: &mut EntityCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            EntityCommandHandlerMsg::Execute {
                command,
                payload,
                reply,
            } => {
                let payload = serde_json::to_string(&payload)?;
                let events = instance.handle(&command, &payload).await?;
                if !events.is_empty() {
                    let sequence = instance.sequence();

                    // Apply events on instance.
                    // We do this before persisting, to make sure nothing blows up,
                    // and it can actually apply the events emitted.
                    let events_to_apply: Vec<_> = events
                        .iter()
                        .map(|event| {
                            let position = sequence.map(|v| v + 1).unwrap_or(0);
                            let event = Event {
                                event: Cow::Borrowed(&event.event),
                                payload: Cow::Borrowed(&event.payload),
                            };
                            (position, event)
                        })
                        .collect();
                    instance.apply(&events_to_apply).await?;

                    // Persist events.
                    let messages: Vec<_> = events
                        .iter()
                        .map(|event| {
                            let payload = serde_json::from_str(&event.payload)?;
                            Ok((event.event.as_ref(), Cow::Owned(payload)))
                        })
                        .collect::<anyhow::Result<_>>()?;
                    let written_messages = stream.write_messages(&messages, sequence)?;

                    if let Err(err) = outbox_relay.cast(OutboxRelayMsg::RelayNextBatch) {
                        error!("failed to notify outbox relay: {err}");
                    }

                    if let Some(reply) = reply {
                        let reply_messages = written_messages
                            .into_iter()
                            .map(|message| message.into_owned())
                            .collect();
                        reply.send(reply_messages)?;
                    }
                } else {
                    if let Some(reply) = reply {
                        reply.send(vec![])?;
                    }
                };
            }
            EntityCommandHandlerMsg::UpdateOutboxActorRef {
                outbox_relay: new_outbox_relay,
            } => *outbox_relay = new_outbox_relay,
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut EntityCommandHandlerState,
    ) -> Result<(), ActorProcessingErr> {
        trace!("dropped entity command handler actor");
        Ok(state.instance.resource_drop().await?)
    }
}
