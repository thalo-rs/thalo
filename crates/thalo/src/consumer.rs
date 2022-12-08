use std::collections::HashSet;

use async_trait::async_trait;
use futures::future::{self};
use futures::{FutureExt, StreamExt};
use message_db::database::{MessageStore, SubscribeToCategoryOpts};
use message_db::message::{GenericMessage, Message, MessageData};
pub use thalo_macros::EventCollection;
use tracing::{info, trace};

pub trait EventCollection: Sized {
    fn entity_names() -> HashSet<&'static str>;
    fn deserialize_event(
        message: GenericMessage,
    ) -> Result<Option<Message<Self>>, serde_json::Error>;
}

#[async_trait]
pub trait EventListener {
    async fn listen(&self, opts: &SubscribeToCategoryOpts) -> anyhow::Result<()>;
}

#[async_trait]
pub trait EventHandler: Sized {
    type Event: EventCollection;

    async fn handle(&self, message: Message<Self::Event>) -> anyhow::Result<()>;
}

pub struct MessageDbEventListener<H> {
    message_store: MessageStore,
    handler: H,
}

impl<H> MessageDbEventListener<H> {
    pub fn new(message_store: MessageStore, handler: H) -> Self {
        MessageDbEventListener {
            message_store,
            handler,
        }
    }
}

#[async_trait]
impl<H> EventListener for MessageDbEventListener<H>
where
    H: EventHandler + Send + Sync + 'static,
    <H as EventHandler>::Event: Send,
{
    async fn listen(&self, opts: &SubscribeToCategoryOpts) -> anyhow::Result<()> {
        let entity_names = H::Event::entity_names();
        let streams = future::join_all(entity_names.iter().map(|entity_name| {
            MessageStore::subscribe_to_category::<MessageData, _>(
                &self.message_store,
                entity_name,
                opts,
            )
            .boxed()
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
        let mut message_stream = futures::stream::select_all(streams);

        while let Some(batch) = message_stream.next().await {
            let messages = batch?;
            for message in messages {
                trace!(?message, "handling message");
                match H::Event::deserialize_event(message)? {
                    Some(event) => {
                        let msg_type = event.msg_type.clone();
                        self.handler.handle(event).await?;
                        info!(msg_type, "handled event");
                    }
                    None => {
                        trace!("ignoring unknown event");
                    }
                }
            }
        }

        Ok(())
    }
}
