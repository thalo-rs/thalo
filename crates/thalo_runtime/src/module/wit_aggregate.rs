mod wit {
    wasmtime::component::bindgen!({
        path: "wit/aggregate.wit",
        world: "aggregate",
        ownership: Borrowing { duplicate_if_necessary: true },
        async: true,
    });
}

use std::borrow::Cow;

pub use wit::exports::aggregate::{Command, Error, EventParam, EventResult};
pub use wit::Aggregate;

impl TryFrom<EventResult> for super::Event<'static> {
    type Error = anyhow::Error;

    fn try_from(event: EventResult) -> Result<Self, Self::Error> {
        Ok(super::Event {
            event: Cow::Owned(event.event),
            payload: Cow::Owned(event.payload),
        })
    }
}
