use std::borrow::Cow;
use std::sync::Arc;

use anyhow::{anyhow, Context, Error, Result};
use futures::StreamExt;
use moka::future::Cache;
use serde_json::Value;
use thalo::event_store::{Event as _, EventStore};
use thalo::stream_name::StreamName;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, trace, warn};
use wasmtime::Trap;

use super::entity_command_handler::EntityCommandHandlerHandle;
use super::CommandGatewayHandle;
// use crate::broadcaster::BroadcasterHandle;
use crate::module::{Event, Module};

#[derive(Clone)]
pub struct AggregateCommandHandlerHandle<E: EventStore> {
    sender: mpsc::Sender<ExecuteAggregateCommand<E>>,
}

impl<E> AggregateCommandHandlerHandle<E>
where
    E: EventStore + Clone + 'static,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync,
{
    pub fn new(
        command_gateway: CommandGatewayHandle<E>,
        name: String,
        event_store: E,
        // broadcaster: BroadcasterHandle,
        module: Module,
        entity_command_handlers: Cache<StreamName<'static>, EntityCommandHandlerHandle<E>>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        tokio::spawn(run_aggregate_command_handler(
            receiver,
            command_gateway,
            name,
            event_store,
            // broadcaster,
            module,
            entity_command_handlers,
        ));

        AggregateCommandHandlerHandle { sender }
    }

    pub async fn execute(
        &self,
        name: String,
        id: String,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<E::Event>, serde_json::Value>> {
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

struct ExecuteAggregateCommand<E: EventStore> {
    name: String,
    id: String,
    command: String,
    payload: Value,
    reply: oneshot::Sender<Result<Result<Vec<E::Event>, serde_json::Value>>>,
}

async fn run_aggregate_command_handler<E>(
    mut receiver: mpsc::Receiver<ExecuteAggregateCommand<E>>,
    command_gateway: CommandGatewayHandle<E>,
    name: String,
    // outbox_relay: OutboxRelayHandle,
    event_store: E,
    // broadcaster: BroadcasterHandle,
    module: Module,
    entity_command_handlers: Cache<StreamName<'static>, EntityCommandHandlerHandle<E>>,
) -> Result<()>
where
    E: EventStore + Clone + 'static,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync,
{
    let handler = AggregateCommandHandler {
        // outbox_relay,
        event_store,
        // broadcaster,
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

struct AggregateCommandHandler<E: EventStore> {
    // outbox_relay: OutboxRelayHandle,
    event_store: E,
    // broadcaster: BroadcasterHandle,
    module: Module,
    entity_command_handlers: Cache<StreamName<'static>, EntityCommandHandlerHandle<E>>,
}

impl<E> AggregateCommandHandler<E>
where
    E: EventStore + Clone + 'static,
    E::Event: Send,
    E::EventStream: Unpin,
    E::Error: Send + Sync,
{
    async fn execute(
        &self,
        name: String,
        id: String,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<E::Event>, serde_json::Value>, (anyhow::Error, Option<Trap>)> {
        let Ok(stream_name) = StreamName::from_parts(name, Some(&id)) else {
            return Err((anyhow!("invalid name or id"), None));
        };

        let entry = self
            .entity_command_handlers
            .entry(stream_name.clone())
            .or_try_insert_with(async {
                let id = stream_name.id().context("missing ID")?;
                let mut instance = self.module.init(&id).await?;
                let mut stream = self.event_store.iter_stream(&stream_name).await?;
                while let Some(res) = stream.next().await {
                    let message = res?;
                    let event = Event {
                        event: message.event_type(),
                        payload: Cow::Owned(serde_json::to_string(&message.data())?),
                    };
                    let sequence = message.sequence();
                    instance.apply(&[(sequence, event)]).await?;
                    trace!(stream_name = ?stream_name, sequence, "applied event");
                }

                let handle = EntityCommandHandlerHandle::new(
                    // self.outbox_relay.clone(),
                    // self.broadcaster.clone(),
                    self.event_store.clone(),
                    stream_name,
                    instance,
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
