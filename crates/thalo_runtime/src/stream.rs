use std::borrow::Cow;

use anyhow::{anyhow, Context as AnyhowContext, Result};
use async_recursion::async_recursion;
use serde_json::Value;
use thalo::event_store::{AppendStreamError, EventStore, NewEvent, StreamEvent as _};
use thalo::stream_name::StreamName;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;
use tracing::trace;

// use crate::broadcaster::BroadcasterHandle;
use crate::module::{Event, Module, ModuleInstance};

pub type StreamHandle<E> = mpsc::Sender<Execute<E>>;

#[derive(Debug)]
pub struct Execute<E>
where
    E: EventStore,
{
    pub command: String,
    pub payload: Value,
    pub max_attempts: u8,
    pub reply: oneshot::Sender<Result<Result<Vec<E::Event>, Value>>>,
}

pub async fn spawn_stream<E>(
    event_store: &E,
    stream_name: &StreamName<'static>,
    module: &Module,
) -> Result<StreamHandle<E>>
where
    E: EventStore + Clone + 'static,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync + 'static,
{
    let id = stream_name.id().context("missing ID")?;
    let mut instance = module.init(&id).await?;
    let mut stream = event_store.iter_stream(&stream_name, 0).await?;
    while let Some(res) = stream.next().await {
        let message = res?;
        let event = Event {
            event: message.event_type(),
            data: Cow::Owned(serde_json::to_string(&message.data())?),
        };
        let sequence = message.sequence();
        instance.apply(&[(sequence, event)]).await?;
        trace!(stream_name = ?stream_name, sequence, "applied event");
    }

    let (sender, receiver) = mpsc::channel(16);
    tokio::spawn(run_stream(
        receiver,
        event_store.clone(),
        stream_name.clone(),
        instance,
    ));

    Ok(sender)
}

async fn run_stream<E>(
    mut receiver: mpsc::Receiver<Execute<E>>,
    event_store: E,
    stream_name: StreamName<'static>,
    instance: ModuleInstance,
) -> Result<()>
where
    E: EventStore,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync + 'static,
{
    let mut stream = Stream {
        event_store,
        stream_name,
        instance,
    };

    while let Some(msg) = receiver.recv().await {
        let res = stream
            .execute(msg.command, msg.payload, msg.max_attempts)
            .await;
        let _ = msg.reply.send(res);
    }

    trace!(stream_name = %stream.stream_name, "stopping stream");
    stream.instance.resource_drop().await?;

    Ok(())
}

struct Stream<E> {
    event_store: E,
    stream_name: StreamName<'static>,
    instance: ModuleInstance,
}

impl<E> Stream<E>
where
    E: EventStore,
    E::Event: Send,
    E::EventStream: Send + Unpin,
    E::Error: Send + Sync + 'static,
{
    async fn execute(
        &mut self,
        command: String,
        payload: Value,
        max_attempts: u8,
    ) -> Result<Result<Vec<E::Event>, Value>> {
        let payload = serde_json::to_string(&payload)?;
        self.execute_retry(command, payload, 0, max_attempts).await
    }

    #[async_recursion]
    async fn execute_retry(
        &mut self,
        command: String,
        payload: String,
        attempt: u8,
        max_attempts: u8,
    ) -> Result<Result<Vec<E::Event>, Value>> {
        let events = match self.instance.handle(&command, &payload).await? {
            Ok(events) => events,
            Err(err) => return Ok(Err(err)),
        };
        if events.is_empty() {
            return Ok(Ok(vec![]));
        }

        let sequence = self.instance.sequence();

        // Persist events.
        let new_events: Vec<_> = events
            .iter()
            .map(|event| {
                let data = serde_json::from_str(&event.data)?;
                Ok(NewEvent::new(&event.event, data))
            })
            .collect::<anyhow::Result<_>>()?;
        let res = self
            .event_store
            .append_to_stream(&self.stream_name, new_events, sequence)
            .await;
        let written_events = match res {
            Ok(events) => events,
            Err(AppendStreamError::Error(err)) => return Err(err.into()),
            Err(err) => {
                let next_attempt = attempt + 1;
                if next_attempt >= max_attempts {
                    return Err(anyhow!("too many wrong expected sequence or conflicts after {next_attempt} retries: {err}"));
                }

                self.apply_new_events().await?;

                return self
                    .execute_retry(command, payload, next_attempt, max_attempts)
                    .await;
            }
        };

        // Apply events on instance.
        let events_to_apply: Vec<_> = events
            .into_iter()
            .enumerate()
            .map(|(i, event)| {
                let position = sequence.map(|v| v + 1 + i as u64).unwrap_or(i as u64);
                let event = Event {
                    event: event.event,
                    data: event.data,
                };
                (position, event)
            })
            .collect();

        self.instance.apply(&events_to_apply).await?;

        Ok(Ok(written_events))
    }

    async fn apply_new_events(&mut self) -> Result<()> {
        let mut stream = self
            .event_store
            .iter_stream(
                &self.stream_name,
                self.instance.sequence().map(|n| n + 1).unwrap_or(0),
            )
            .await?;

        while let Some(res) = stream.next().await {
            let message = res?;
            let event = Event {
                event: message.event_type(),
                data: Cow::Owned(serde_json::to_string(&message.data())?),
            };
            let sequence = message.sequence();
            self.instance.apply(&[(sequence, event)]).await?;
            trace!(stream_name = ?self.stream_name, sequence, "applied event after conflict");
        }

        Ok(())
    }
}
