use std::borrow::Cow;

use anyhow::{Context as AnyhowContext, Result};
use serde_json::Value;
use thalo::stream_name::StreamName;
use thalo_message_store::message::Message;
use thalo_message_store::stream::Stream;
use thalo_message_store::MessageStore;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, trace};

use super::outbox_relay::OutboxRelayHandle;
use crate::broadcaster::BroadcasterHandle;
use crate::module::{Event, Module, ModuleInstance};

#[derive(Clone)]
pub struct EntityCommandHandlerHandle {
    sender: mpsc::Sender<ExecuteEntityCommand>,
}

#[derive(Debug)]
struct ExecuteEntityCommand {
    command: String,
    payload: Value,
    reply: oneshot::Sender<Result<Result<Vec<Message<'static>>, serde_json::Value>>>,
}

impl EntityCommandHandlerHandle {
    pub fn new(
        outbox_relay: OutboxRelayHandle,
        message_store: MessageStore,
        broadcaster: BroadcasterHandle,
        module: Module,
        stream_name: StreamName<'static>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(16);
        tokio::spawn(run_entity_command_handler(
            receiver,
            outbox_relay,
            message_store,
            broadcaster,
            module,
            stream_name,
        ));

        EntityCommandHandlerHandle { sender }
    }

    pub async fn execute(
        &self,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<Message<'static>>, serde_json::Value>> {
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

async fn run_entity_command_handler(
    mut receiver: mpsc::Receiver<ExecuteEntityCommand>,
    outbox_relay: OutboxRelayHandle,
    message_store: MessageStore,
    broadcaster: BroadcasterHandle,
    module: Module,
    stream_name: StreamName<'static>,
) -> Result<()> {
    let mut instance = module
        .init(&stream_name.id().context("missing ID")?)
        .await?;
    let stream = message_store.stream(stream_name)?;
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

    let mut handler = EntityCommandHandler {
        outbox_relay,
        broadcaster,
        stream,
        instance,
    };

    while let Some(msg) = receiver.recv().await {
        let res = handler.execute(msg.command, msg.payload).await;
        let _ = msg.reply.send(res);
    }

    trace!(stream_name = %handler.stream.stream_name(), "stopping entity command handler");
    handler.instance.resource_drop().await?;

    Ok(())
}

struct EntityCommandHandler {
    outbox_relay: OutboxRelayHandle,
    broadcaster: BroadcasterHandle,
    stream: Stream<'static>,
    instance: ModuleInstance,
}

impl EntityCommandHandler {
    async fn execute(
        &mut self,
        command: String,
        payload: Value,
    ) -> Result<Result<Vec<Message<'static>>, serde_json::Value>> {
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
        let messages: Vec<_> = events
            .iter()
            .map(|event| {
                let payload = serde_json::from_str(&event.payload)?;
                Ok((event.event.as_ref(), Cow::Owned(payload)))
            })
            .collect::<anyhow::Result<_>>()?;
        let written_messages = self.stream.write_messages(&messages, sequence)?;

        for message in &written_messages {
            if let Err(err) = self
                .broadcaster
                .broadcast_event(message.clone().into_owned())
                .await
            {
                error!("failed to broadcast event: {err}");
            }
        }

        if let Err(err) = self.outbox_relay.relay_next_batch().await {
            error!("failed to notify outbox relay: {err}");
        }

        let reply_messages = written_messages
            .into_iter()
            .map(|message| message.into_owned())
            .collect();
        Ok(Ok(reply_messages))
    }
}
