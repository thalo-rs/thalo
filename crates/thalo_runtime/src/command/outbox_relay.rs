//! Outbox Relay for Event Distribution
//!
//! The outbox relay facilitates the transfer of events to external streams, such as Redis. It enables event handlers to
//! efficiently receive and process these events.
//!
//! This implementation dispatches events in batches, with each batch containing a maximum of 100 events. Events are
//! relayed to a pre-configured external system for further handling.

use std::time::Duration;

use async_trait::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use thalo::Category;
use thalo_message_store::{message::MessageData, outbox::Outbox};

use crate::{
    flusher::{Flusher, FlusherMsg, FlusherRef},
    relay::Relay,
};

const BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

pub type OutboxRelayRef = ActorRef<OutboxRelayMsg>;

pub struct OutboxRelay;

pub struct OutboxRelayState {
    outbox: Outbox,
    relay: Relay,
    stream_name: String,
    flusher: FlusherRef,
}

pub enum OutboxRelayMsg {
    RelayNextBatch,
}

pub struct OutboxRelayArgs {
    pub category: Category<'static>,
    pub outbox: Outbox,
    pub relay: Relay,
}

#[async_trait]
impl Actor for OutboxRelay {
    type State = OutboxRelayState;
    type Msg = OutboxRelayMsg;
    type Arguments = OutboxRelayArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        OutboxRelayArgs {
            category,
            outbox,
            relay,
        }: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let stream_name = relay.stream_name(category.as_borrowed());

        let (flusher, _) = Flusher::spawn_linked(
            Some(format!("{category}_outbox_flusher")),
            myself.get_cell(),
            FLUSH_INTERVAL,
            {
                let outbox = outbox.clone();
                move || {
                    let outbox = outbox.clone();
                    async move {
                        outbox.flush_async().await?;
                        Ok(())
                    }
                }
            },
        )
        .await?;

        myself.cast(OutboxRelayMsg::RelayNextBatch)?;

        Ok(OutboxRelayState {
            outbox,
            relay,
            stream_name,
            flusher,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        _msg: Self::Msg,
        OutboxRelayState {
            outbox,
            relay,
            stream_name,
            flusher,
        }: &mut OutboxRelayState,
    ) -> Result<(), ActorProcessingErr> {
        let batch = outbox.iter_all_messages::<MessageData>().take(BATCH_SIZE);

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

        relay.relay(stream_name, messages).await?;
        outbox.delete_batch(keys)?;

        flusher.cast(FlusherMsg::MarkDirty)?;

        // If the current batch is equal to BATCH_SIZE,
        // then it's likely there's more messages which need to be relayed.
        if size == BATCH_SIZE {
            myself.cast(OutboxRelayMsg::RelayNextBatch)?;
        }

        Ok(())
    }
}
