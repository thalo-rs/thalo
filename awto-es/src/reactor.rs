use std::collections::HashMap;

use crate::Error;

/// EventHandler must run once only when multiple nodes of the
/// application are running at the same time (via locks in the database).
///
/// They keep track of their latest sequence and only process events that
/// have not yet been processed yet.
pub trait Reactor {
    type Event;

    fn handle(
        &mut self,
        event: Self::Event,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<(), Error>;
}
