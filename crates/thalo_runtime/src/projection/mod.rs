mod projection_gateway;
mod projection_subscription;

use async_trait::async_trait;
pub use projection_gateway::*;
use thalo_message_store::message::Message;

#[async_trait]
pub trait Projection {
    type Event;

    async fn handle(&mut self, message: Message<'static, Self::Event>) -> anyhow::Result<()>;
    async fn last_global_id(&self) -> anyhow::Result<Option<u64>>;
}
