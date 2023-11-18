//! Ensures events are broadcasted in the correct order.

use std::collections::HashMap;

use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use thalo_message_store::message::GenericMessage;
use tokio::sync::broadcast::{self, error::SendError};

pub type BroadcasterRef = ActorRef<BroadcasterMsg>;

pub struct Broadcaster;

pub struct BroadcasterState {
    tx: broadcast::Sender<GenericMessage<'static>>,
    buffer: HashMap<u64, GenericMessage<'static>>,
    expected_next_id: u64,
}

pub enum BroadcasterMsg {
    NewEvent(GenericMessage<'static>),
}

pub struct BroadcasterArgs {
    pub tx: broadcast::Sender<GenericMessage<'static>>,
    pub last_position: Option<u64>,
}

#[async_trait]
impl Actor for Broadcaster {
    type State = BroadcasterState;
    type Msg = BroadcasterMsg;
    type Arguments = BroadcasterArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        BroadcasterArgs { tx, last_position }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(BroadcasterState {
            tx,
            buffer: HashMap::new(),
            expected_next_id: last_position.unwrap_or(0),
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut BroadcasterState,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            BroadcasterMsg::NewEvent(event) => {
                state.buffer.insert(event.global_id, event);
                state.process_buffer()?;
            }
        }

        Ok(())
    }
}

impl BroadcasterState {
    fn process_buffer(&mut self) -> Result<(), SendError<GenericMessage<'static>>> {
        while let Some(event) = self.buffer.remove(&self.expected_next_id) {
            self.broadcast_event(event)?;
            self.expected_next_id += 1;
        }

        Ok(())
    }

    fn broadcast_event(
        &self,
        event: GenericMessage<'static>,
    ) -> Result<usize, SendError<GenericMessage<'static>>> {
        self.tx.send(event)
    }
}
