//! Ensures events are broadcasted in the correct order.

use std::collections::HashMap;

use anyhow::{Context, Result};
use thalo::event_store::message::Message;
use tokio::sync::{broadcast, mpsc};
use tracing::error;

#[derive(Clone)]
pub struct BroadcasterHandle {
    sender: mpsc::Sender<Message<'static>>,
}

impl BroadcasterHandle {
    pub fn new(tx: broadcast::Sender<Message<'static>>, last_position: Option<u64>) -> Self {
        let (sender, receiver) = mpsc::channel(64);
        tokio::spawn(run_broadcaster(receiver, tx, last_position));

        BroadcasterHandle { sender }
    }

    pub async fn broadcast_event(&self, event: Message<'static>) -> Result<()> {
        self.sender
            .send(event)
            .await
            .context("broadcaster is not running")
    }
}

async fn run_broadcaster(
    mut receiver: mpsc::Receiver<Message<'static>>,
    tx: broadcast::Sender<Message<'static>>,
    last_position: Option<u64>,
) -> Result<()> {
    let mut broadcaster = Broadcaster {
        tx,
        buffer: HashMap::new(),
        expected_next_id: last_position.unwrap_or(0) + 1,
    };

    while let Some(event) = receiver.recv().await {
        if let Err(err) = broadcaster.broadcast_event(event) {
            error!("failed to broadcast message: {err}");
        }
    }

    Ok(())
}

struct Broadcaster {
    tx: broadcast::Sender<Message<'static>>,
    buffer: HashMap<u64, Message<'static>>,
    expected_next_id: u64,
}

impl Broadcaster {
    fn broadcast_event(&mut self, event: Message<'static>) -> Result<()> {
        self.buffer.insert(event.global_sequence, event);
        self.process_buffer()
    }

    fn process_buffer(&mut self) -> Result<()> {
        while let Some(event) = self.buffer.remove(&self.expected_next_id) {
            self.tx.send(event)?;
            self.expected_next_id += 1;
        }

        Ok(())
    }
}
