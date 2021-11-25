//! # Thalo
//!
//! Thalo is a framework for building large-scale event sourced applications with CQRS.
//!
//! It is being built to simplify the process of building event driven applications
//! by simpifying the API with macros and providing the backbone, allowing developers
//! to focus on their business requirements.
//!
//! The goal is to make this powerful approach of developing much more accessible to developers.
//!
//! ## Current status
//!
//! **Thalo is still under heavy development and is not production ready.**
//! 
//! Documentation is extremely lacking, but will improve once the API begins
//! to stabilise.
//! 
//! ## Get in touch
//!
//! If you'd like to ask/discuss or learn more, you can reach out via Discord.
//! <https://github.com/Acidic9>
//!
//! ## Example
//!
//! An example can be seen at <https://github.com/awto-rs/thalo/tree/main/examples/bank>.

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
