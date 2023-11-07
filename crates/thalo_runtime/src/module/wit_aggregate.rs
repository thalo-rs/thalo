mod wit {
    wasmtime::component::bindgen!({
        path: "../../wit/aggregate.wit",
        world: "aggregate",
        ownership: Borrowing { duplicate_if_necessary: true },
        async: true,
    });
}

use std::borrow::Cow;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use thalo::StreamName;
pub use wit::Aggregate;
// pub use wit::exports::aggregate::Aggregate;
pub use wit::exports::aggregate::Command;
pub use wit::exports::aggregate::ContextParam;
pub use wit::exports::aggregate::ContextResult;
pub use wit::exports::aggregate::Error;
pub use wit::exports::aggregate::EventParam;
pub use wit::exports::aggregate::EventResult;

impl<'a> From<(&'a thalo::Context<'_>, &'a str)> for ContextParam<'a> {
    fn from((ctx, metadata): (&'a thalo::Context<'_>, &'a str)) -> Self {
        ContextParam {
            id: ctx.id,
            stream_name: &ctx.stream_name,
            position: ctx.position,
            metadata,
            time: ctx
                .time
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

impl TryFrom<thalo::Context<'_>> for ContextResult {
    type Error = anyhow::Error;

    fn try_from(ctx: thalo::Context<'_>) -> Result<Self, Self::Error> {
        Ok(ContextResult {
            id: ctx.id,
            stream_name: ctx.stream_name.into(),
            position: ctx.position,
            metadata: serde_json::to_string(&ctx.metadata)?,
            time: ctx
                .time
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }
}

impl TryFrom<ContextResult> for thalo::Context<'_> {
    type Error = anyhow::Error;

    fn try_from(ctx: ContextResult) -> Result<Self, Self::Error> {
        let stream_name = StreamName::new(ctx.stream_name)?;
        let metadata = serde_json::from_str(&ctx.metadata)?;
        Ok(thalo::Context {
            id: ctx.id,
            stream_name,
            position: ctx.position,
            metadata,
            time: UNIX_EPOCH + Duration::from_millis(ctx.time),
        })
    }
}

impl TryFrom<EventResult> for super::Event<'static> {
    type Error = anyhow::Error;

    fn try_from(event: EventResult) -> Result<Self, Self::Error> {
        Ok(super::Event {
            ctx: event.ctx.try_into()?,
            event: Cow::Owned(event.event),
            payload: Cow::Owned(event.payload),
        })
    }
}
