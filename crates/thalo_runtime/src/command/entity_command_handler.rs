use std::borrow::Cow;

use anyhow::{Context as AnyhowContext, Result};
use serde_json::Value;
use thalo::event_store::{EventStore, NewEvent};
use thalo::stream_name::StreamName;
use tokio::sync::{mpsc, oneshot};
use tracing::trace;

// use crate::broadcaster::BroadcasterHandle;
use crate::module::{Event, ModuleInstance};

#[derive(Clone)]
pub struct EntityCommandHandlerHandle<E: EventStore> {
    sender: mpsc::Sender<ExecuteEntityCommand<E>>,
}

#[derive(Debug)]
struct ExecuteEntityCommand<E>
where
    E: EventStore,
{
    command: String,
    payload: Value,
    reply: oneshot::Sender<Result<Result<Vec<E::Event>, serde_json::Value>>>,
}

impl<E> EntityCommandHandlerHandle<E>
where
    E: EventStore + Clone + 'static,
    E::Event: Send,
    E::Error: Send + Sync,
{
    pub fn new(
        // outbox_relay: OutboxRelayHandle,
        // broadcaster: BroadcasterHandle,
        event_store: E,
        stream_name: StreamName<'static>,
        instance: ModuleInstance,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        tokio::spawn(run_entity_command_handler(
            receiver,
            // outbox_relay,
            // broadcaster,
            event_store,
            stream_name,
            instance,
        ));

        EntityCommandHandlerHandle { sender }
    }

    pub async fn execute(
        &self,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<E::Event>, serde_json::Value>> {
        let (reply, recv) = oneshot::channel();
        let msg = ExecuteEntityCommand {
            command,
            payload,
            reply,
        };

        let _ = self.sender.send(msg).await;
        recv.await
            .context("no response from entity command handler")?
    }
}

async fn run_entity_command_handler<E>(
    mut receiver: mpsc::Receiver<ExecuteEntityCommand<E>>,
    // outbox_relay: OutboxRelayHandle,
    // broadcaster: BroadcasterHandle,
    event_store: E,
    stream_name: StreamName<'static>,
    instance: ModuleInstance,
) -> Result<()>
where
    E: EventStore + Clone,
    E::Error: Send + Sync + 'static,
{
    let mut handler = EntityCommandHandler {
        // outbox_relay,
        // broadcaster,
        event_store,
        stream_name,
        instance,
    };

    while let Some(msg) = receiver.recv().await {
        let res = handler.execute(msg.command, msg.payload).await;
        let _ = msg.reply.send(res);
    }

    trace!(stream_name = %handler.stream_name, "stopping entity command handler");
    handler.instance.resource_drop().await?;

    Ok(())
}

struct EntityCommandHandler<E> {
    // outbox_relay: OutboxRelayHandle,
    // broadcaster: BroadcasterHandle,
    event_store: E,
    stream_name: StreamName<'static>,
    instance: ModuleInstance,
}

impl<E> EntityCommandHandler<E>
where
    E: EventStore + Clone,
    E::Error: Send + Sync + 'static,
{
    async fn execute(
        &mut self,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<E::Event>, serde_json::Value>> {
        let payload = serde_json::to_string(&payload)?;
        let events = match self.instance.handle(&command, &payload).await? {
            Ok(events) => events,
            Err(err) => return Ok(Err(err)),
        };
        if events.is_empty() {
            return Ok(Ok(vec![]));
        }

        let sequence = self.instance.sequence();

        // Apply events on instance.
        // We do this before persisting, to make sure nothing blows up,
        // and it can actually apply the events emitted.
        let events_to_apply: Vec<_> = events
            .iter()
            .enumerate()
            .map(|(i, event)| {
                let position = sequence.map(|v| v + 1 + i as u64).unwrap_or(i as u64);
                let event = Event {
                    event: Cow::Borrowed(&event.event),
                    payload: Cow::Borrowed(&event.payload),
                };
                (position, event)
            })
            .collect();
        self.instance.apply(&events_to_apply).await?;

        // Persist events.
        let new_events: Vec<_> = events
            .iter()
            .map(|event| {
                let data = serde_json::from_str(&event.payload)?;
                Ok(NewEvent::new(event.event.to_string(), data))
            })
            .collect::<anyhow::Result<_>>()?;
        let written_messages = self
            .event_store
            .append_to_stream(&self.stream_name, new_events, sequence)
            .await?;

        // for message in &written_messages {
        //     if let Err(err) = self
        //         .broadcaster
        //         .broadcast_event(message.clone().into_owned())
        //         .await
        //     {
        //         error!("failed to broadcast event: {err}");
        //     }
        // }

        // if let Err(err) = self.outbox_relay.relay_next_batch().await {
        //     error!("failed to notify outbox relay: {err}");
        // }

        // let reply_messages = written_messages
        //     .into_iter()
        //     .map(|message| message.into_owned())
        //     .collect();

        Ok(Ok(written_messages))
    }
}
