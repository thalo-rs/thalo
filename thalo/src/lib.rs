use app::*;
#[doc(hidden)]
pub use thalo_macros::*;
#[doc(inline)]
pub use command::*;
pub use error::Error;
pub use topic::*;
#[doc(inline)]
pub use query::*;

mod app;
mod command;
mod error;
mod topic;
pub mod postgres;
mod query;

/// Builds a thalo app.
pub fn build<ES>(event_store: ES, redpanda_host: impl Into<String> + Clone) -> AppBuilder<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    App::build(event_store, redpanda_host)
}
