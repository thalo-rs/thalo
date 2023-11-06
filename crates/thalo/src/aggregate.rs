use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::command::Commands;
use crate::{Error, Events};

pub trait Aggregate
where
    Self: Serialize + DeserializeOwned,
{
    type Event: Events<Aggregate = Self>;
    type Command: Commands<Aggregate = Self>;

    fn aggregate_type() -> &'static str;
    fn new(id: String) -> Result<Self, Error>;
}

#[doc(hidden)]
pub mod wit_aggregate {
    wit_bindgen::generate!("../../wit/aggregate.wit");

    pub use aggregate::*;
    use chrono::{TimeZone, Utc};

    use crate::Events;

    impl<T> aggregate::Aggregate for T
    where
        T: super::Aggregate,
    {
        fn init(id: String) -> Result<State, Error> {
            let state = T::new(id)?;
            serde_json::to_vec(&state).map_err(|err| Error::SerializeState(err.to_string()))
        }

        fn apply(state: State, events: Vec<Event>) -> Result<State, Error> {
            let mut state: T = serde_json::from_slice(&state)
                .map_err(|err| Error::DeserializeState(err.to_string()))?;
            for event in events {
                <T::Event as super::Events>::apply(
                    &mut state,
                    event.ctx.try_into()?,
                    &event.event_type,
                    event.payload,
                )?;
            }
            serde_json::to_vec(&state).map_err(|err| Error::SerializeState(err.to_string()))
        }

        fn handle(state: State, ctx: Context, command: Command) -> Result<Vec<Event>, Error> {
            let state: T = serde_json::from_slice(&state)
                .map_err(|err| Error::DeserializeState(err.to_string()))?;
            let mut ctx = ctx.try_into()?;
            let events = <T::Command as super::Commands>::handle(
                &state,
                &mut ctx,
                &command.command,
                command.payload,
            )?;
            events
                .into_iter()
                .map(|event| {
                    Result::<_, Error>::Ok(Event {
                        ctx: ctx.wit_context(),
                        event_type: event.event_type().to_string(),
                        payload: event.payload()?,
                    })
                })
                .collect()
        }
    }

    impl TryFrom<Context> for crate::Context {
        type Error = Error;

        fn try_from(ctx: Context) -> Result<Self, Self::Error> {
            let id = ctx
                .id
                .parse()
                .map_err(|err: uuid::Error| Error::DeserializeContext(err.to_string()))?;
            let stream_name = ctx
                .stream_name
                .parse()
                .map_err(|err: message_db::Error| Error::DeserializeContext(err.to_string()))?;
            let metadata = serde_json::from_slice(&ctx.metadata)
                .map_err(|err| Error::DeserializeContext(err.to_string()))?;
            let Some(time) = Utc.timestamp_opt(ctx.time / 1_000, (ctx.time % 1_000_000) as u32).single() else {
                return Err(Error::DeserializeContext("invalid timestamp".to_string()));
            };

            Ok(crate::Context {
                id,
                stream_name,
                position: ctx.position,
                global_position: ctx.global_position,
                metadata,
                time,
            })
        }
    }

    impl crate::Context {
        fn wit_context(&self) -> Context {
            Context {
                id: self.id.to_string(),
                stream_name: self.stream_name.to_string(),
                position: self.position,
                global_position: self.global_position,
                metadata: serde_json::to_vec(&self.metadata).unwrap(),
                time: self.time.timestamp_millis(),
            }
        }
    }
}
