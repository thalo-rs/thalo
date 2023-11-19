//! Outbox Relay for Event Distribution
//!
//! The outbox relay facilitates the transfer of events to external streams, such as Redis. It enables event handlers to
//! efficiently receive and process these events.
//!
//! This implementation dispatches events in batches, with each batch containing a maximum of 100 events. Events are
//! relayed to a pre-configured external system for further handling.

use std::{pin::Pin, time::Duration};

use anyhow::{Context, Result};
use futures::Future;
use thalo::Category;
use thalo_message_store::{message::MessageData, outbox::Outbox};
use tokio::{sync::mpsc, time::interval};
use tracing::{error, warn};

use crate::relay::Relay;

const BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct OutboxRelayHandle {
    sender: mpsc::Sender<()>,
}

impl OutboxRelayHandle {
    pub fn new(name: Category<'static>, outbox: Outbox, relay: Relay) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        tokio::spawn(run_outbox_relay(receiver, name, outbox, relay));

        OutboxRelayHandle { sender }
    }

    pub async fn relay_next_batch(&self) -> Result<()> {
        self.sender
            .send(())
            .await
            .context("outbox relay is not running")
    }
}

async fn run_outbox_relay(
    mut receiver: mpsc::Receiver<()>,
    name: Category<'static>,
    outbox: Outbox,
    relay: Relay,
) {
    let stream_name = relay.stream_name(name.as_borrowed());

    let mut outbox_relay = OutboxRelay {
        outbox,
        relay,
        stream_name,
        is_dirty: false,
    };

    if let Err(err) = outbox_relay.relay_next_batch().await {
        error!("{err}");
    }

    let mut timer = interval(FLUSH_INTERVAL);

    loop {
        tokio::select! {
            msg = receiver.recv() => match msg {
                Some(()) => {
                    if let Err(err) = outbox_relay.relay_next_batch().await {
                        error!("{err}");
                    }
                }
                None => break,
            },
            _ = timer.tick() => {
                if let Err(err) = outbox_relay.flush().await {
                    error!("{err}");
                }
            }
        }
    }

    warn!(%name, "outbox relay stopping")
}

struct OutboxRelay {
    outbox: Outbox,
    relay: Relay,
    stream_name: String,
    is_dirty: bool,
}

impl OutboxRelay {
    fn relay_next_batch<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let batch = self
                .outbox
                .iter_all_messages::<MessageData>()
                .take(BATCH_SIZE);

            let size_hint = batch.size_hint().0;
            let mut keys = Vec::with_capacity(size_hint);
            let mut messages = Vec::with_capacity(size_hint);
            for res in batch {
                let raw_message = res?;
                let message = raw_message.message()?.into_owned();
                let key = raw_message.key;

                keys.push(key);
                messages.push(message);
            }

            debug_assert_eq!(keys.len(), messages.len());
            let size = keys.len();

            self.relay.relay(&self.stream_name, messages).await?;
            self.outbox.delete_batch(keys)?;

            self.is_dirty = true;

            // If the current batch is equal to BATCH_SIZE,
            // then it's likely there's more messages which need to be relayed.
            if size == BATCH_SIZE {
                self.relay_next_batch().await?;
            }

            Ok(())
        })
    }

    async fn flush(&mut self) -> Result<()> {
        if self.is_dirty {
            self.is_dirty = false;
            self.outbox.flush_async().await?;
        }

        Ok(())
    }
}
