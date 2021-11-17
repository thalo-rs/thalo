use async_trait::async_trait;

use crate::{Error, EventHandler};

#[async_trait]
pub trait Projection: EventHandler {
    async fn last_event_id(&self) -> Result<Option<i64>, Error>;
}
