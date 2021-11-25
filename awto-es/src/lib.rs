use awto::*;
#[doc(hidden)]
pub use awto_es_macros::*;
#[doc(inline)]
pub use command::*;
pub use error::Error;
pub use topic::*;
#[doc(inline)]
pub use query::*;

mod awto;
mod command;
mod error;
mod topic;
pub mod postgres;
mod query;

/// Builds an awto app.
pub fn build<ES>(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AwtoBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    Awto::build(event_store, redpanda_host)
}
