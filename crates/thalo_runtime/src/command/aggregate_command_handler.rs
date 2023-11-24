use anyhow::{anyhow, Context, Result};
use moka::future::Cache;
use serde_json::Value;
use thalo::stream_name::{Category, StreamName, ID};
use thalo_message_store::message::Message;
use thalo_message_store::MessageStore;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use super::entity_command_handler::EntityCommandHandlerHandle;
use super::outbox_relay::OutboxRelayHandle;
use crate::broadcaster::BroadcasterHandle;
use crate::module::Module;

#[derive(Clone)]
pub struct AggregateCommandHandlerHandle {
    sender: mpsc::Sender<ExecuteAggregateCommand>,
}

impl AggregateCommandHandlerHandle {
    pub fn new(
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
    ) -> Result<Vec<Message<'static>>> {
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
    reply: oneshot::Sender<Result<Vec<Message<'static>>>>,
}

async fn run_aggregate_command_handler(
    mut receiver: mpsc::Receiver<ExecuteAggregateCommand>,
    name: Category<'static>,
    outbox_relay: OutboxRelayHandle,
    message_store: MessageStore,
    broadcaster: BroadcasterHandle,
    cache_size: u64,
    module: Module,
) {
    let entity_command_handlers = Cache::new(cache_size);

    let mut handler = AggregateCommandHandler {
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
        let _ = msg.reply.send(res);
    }

    warn!(%name, "aggregate command handler stopping");
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
        &mut self,
        name: Category<'static>,
        id: ID<'static>,
        command: String,
        payload: Value,
    ) -> Result<Vec<Message<'static>>> {
        let Ok(stream_name) = StreamName::from_parts(name, Some(&id)) else {
            return Err(anyhow!("invalid name or id"));
        };
        let entry = self
            .entity_command_handlers
            .entry(stream_name.clone())
            .or_insert_with(async {
                EntityCommandHandlerHandle::new(
                    self.outbox_relay.clone(),
                    self.message_store.clone(),
                    self.broadcaster.clone(),
                    self.module.clone(),
                    stream_name,
                )
            })
            .await;

        entry.value().execute(command, payload).await
    }
}
