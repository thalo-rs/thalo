use async_trait::async_trait;

use crate::{Error, EventHandler};

#[async_trait]
pub trait Projection: EventHandler {
    fn projection_type() -> &'static str;

    async fn last_event_id(&self) -> Result<Option<i64>, Error>;
}
