use std::borrow::Cow;
use std::sync::Arc;

use anyhow::{anyhow, Context, Error, Result};
use moka::future::Cache;
use serde_json::Value;
use thalo::stream_name::{Category, StreamName, ID};
use thalo_message_store::message::Message;
use thalo_message_store::MessageStore;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, trace, warn};
use wasmtime::Trap;

use super::entity_command_handler::EntityCommandHandlerHandle;
use super::outbox_relay::OutboxRelayHandle;
use super::CommandGatewayHandle;
use crate::broadcaster::BroadcasterHandle;
use crate::module::{Event, Module};

#[derive(Clone)]
pub struct AggregateCommandHandlerHandle {
    sender: mpsc::Sender<ExecuteAggregateCommand>,
}

impl AggregateCommandHandlerHandle {
    pub fn new(
        command_gateway: CommandGatewayHandle,
        name: Category<'static>,
        outbox_relay: OutboxRelayHandle,
        message_store: MessageStore,
        broadcaster: BroadcasterHandle,
        cache_size: u64,
        module: Module,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        tokio::spawn(run_aggregate_command_handler(
            receiver,
            command_gateway,
            name,
            outbox_relay,
            message_store,
            broadcaster,
            cache_size,
            module,
        ));

        AggregateCommandHandlerHandle { sender }
    }

    pub async fn execute(
        &self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<Message<'static>>, serde_json::Value>> {
        let (reply, recv) = oneshot::channel();
        let msg = ExecuteAggregateCommand {
            name,
            id,
            command,
            payload,
            reply,
        };

        let _ = self.sender.send(msg).await;
        recv.await.context("no response from command handler")?
    }
}

struct ExecuteAggregateCommand {
    name: Category<'static>,
    id: ID<'static>,
    command: String,
    payload: Value,
    reply: oneshot::Sender<Result<Result<Vec<Message<'static>>, serde_json::Value>>>,
}

async fn run_aggregate_command_handler(
    mut receiver: mpsc::Receiver<ExecuteAggregateCommand>,
    command_gateway: CommandGatewayHandle,
    name: Category<'static>,
    outbox_relay: OutboxRelayHandle,
    message_store: MessageStore,
    broadcaster: BroadcasterHandle,
    cache_size: u64,
    module: Module,
) -> Result<()> {
    let entity_command_handlers = Cache::new(cache_size);

    let handler = AggregateCommandHandler {
        outbox_relay,
        message_store,
        broadcaster,
        module,
        entity_command_handlers,
    };

    while let Some(msg) = receiver.recv().await {
        let res = handler
            .execute(msg.name, msg.id, msg.command, msg.payload)
            .await;
        let res = match res {
            Ok(res) => Ok(res),
            Err((err, None)) => Err(err),
            Err((err, Some(trap))) => {
                error!("aggregate trapped: {trap}");
                let _ = msg.reply.send(Err(err));
                break;
            }
        };

        let _ = msg.reply.send(res);
    }

    warn!(%name, "aggregate command handler restarting");

    let module = handler.module.new_instance().await?;
    command_gateway.start_module_from_module(name, module).await
}

struct AggregateCommandHandler {
    outbox_relay: OutboxRelayHandle,
    message_store: MessageStore,
    broadcaster: BroadcasterHandle,
    module: Module,
    entity_command_handlers: Cache<StreamName<'static>, EntityCommandHandlerHandle>,
}

impl AggregateCommandHandler {
    async fn execute(
        &self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<Message<'static>>, serde_json::Value>, (anyhow::Error, Option<Trap>)>
    {
        let Ok(stream_name) = StreamName::from_parts(name, Some(&id)) else {
            return Err((anyhow!("invalid name or id"), None));
        };

        let entry = self
            .entity_command_handlers
            .entry(stream_name.clone())
            .or_try_insert_with(async {
                let id = stream_name.id().context("missing ID")?;
                let mut instance = self.module.init(&id).await?;
                let stream = self.message_store.stream(stream_name)?;
                for res in stream.iter_all_messages::<()>() {
                    let raw_message = res?;
                    let message = raw_message.message()?;
                    let event = Event {
                        event: message.msg_type,
                        payload: Cow::Owned(serde_json::to_string(&message.data)?),
                    };
                    instance.apply(&[(message.position, event)]).await?;
                    trace!(stream_name = ?stream.stream_name(), position = message.position, "applied event");
                }

                let handle = EntityCommandHandlerHandle::new(
                    self.outbox_relay.clone(),
                    self.broadcaster.clone(),
                    instance,
                    stream,
                );

                Ok(handle)
            })
            .await
            .map_err(|err: Arc<Error>| {
                (anyhow!("{err}"), err.root_cause().downcast_ref().copied())
            })?;

        entry
            .value()
            .execute(command, payload)
            .await
            .map_err(|err| {
                let trap = err.root_cause().downcast_ref().copied();
                (err, trap)
            })
    }
}
