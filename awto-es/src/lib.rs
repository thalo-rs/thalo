use awto::{Awto, AwtoBuilder};
pub use awto_es_macros as macros;
pub use command::*;
pub use error::Error;
pub use message::*;
pub use query::*;

pub mod awto;
pub mod command;
mod error;
mod message;
pub mod postgres;
pub mod query;

pub fn build<ES>(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AwtoBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    Awto::build(event_store, redpanda_host)
}
