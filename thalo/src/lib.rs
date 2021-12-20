//! A framework for building large-scale event sourced systems with CQRS.
//!
//! Thalo is a framework for creating applications with an event driven approach following
//! concepts including [**event sourcing**](https://microservices.io/patterns/data/event-sourcing.html), [**CQRS**](https://microservices.io/patterns/data/cqrs.html), [**transactional outbox**](https://microservices.io/patterns/data/transactional-outbox.html), [**event driven**](https://martinfowler.com/articles/201701-event-driven.html), [**DDD**](https://martinfowler.com/bliki/DomainDrivenDesign.html).
//!
//! It is built around some core features including:
//! - Event Store
//! - Aggregates
//! - Commands & Events
//! - Projections & View Models
//! - Actors
//!
//! The goal of the framrwork is to simplify the process of building event driven applications
//! by providing an opinionated approach through macros and providing the backbone,
//! allowing developers to focus on their business requirements.
//!
//! ## Current status
//!
//! **Thalo is still under heavy development and is not production ready.**
//!
//! Documentation is lacking, but will improve once the API begins to stabilise.
//!
//! ## Get in touch
//!
//! If you'd like to ask/discuss or learn more, you can reach via the liks below:
//!
//! - [Github](https://github.com/tqwewe)
//! - [Discord Server](https://discord.gg/4Cq8NnPYPA)
//!
//! ## Example
//!
//! The following is an example of a bank, showing how to define a single command and event.
//!
//! ```
//! use thalo::{commands, events, Aggregate, AggregateType, Error};
//!
//! #[derive(Aggregate, Clone, Debug, Default)]
//! pub struct BankAccount {
//!     #[identity]
//!     account_number: String,
//!     balance: f64,
//!     opened: bool,
//! }
//!
//! #[commands]
//! impl BankAccount {
//!     /// Creates a command for opening an account
//!     pub fn open_account(&self, initial_balance: f64) -> Result<AccountOpenedEvent, Error> {
//!         if self.opened {
//!             return Err(Error::invariant_code("ACCOUNT_ALREADY_OPENED"));
//!         }
//!
//!         // Reference the event created by the BankAccount::account_opened method
//!         Ok(AccountOpenedEvent { initial_balance })
//!     }
//! }
//!
//! #[events]
//! impl BankAccount {
//!     /// Creates an event for when a user opened an account
//!     pub fn account_opened(&mut self, initial_balance: f64) {
//!         self.balance = initial_balance;
//!         self.opened = true;
//!     }
//! }
//! ```
//!
//! The `#[commands]` and `#[events]` attribute macros generate a lot based on your implementations including:
//!
//! - `BankAccountCommand` enum
//! - `BankAccountEvent` enum
//! - `AccountOpenedEvent` struct
//!
//! _Below are the enums & structs generated:_
//!
//! ```
//! pub enum BankAccountCommand {
//!     OpenAccount {
//!         initial_balance: f64,
//!     },
//! }
//!
//! pub enum BankAccountEvent {
//!     AccountOpened(AccountOpenedEvent),
//! }
//!
//! pub struct AccountOpenedEvent {
//!     initial_balance: f64,
//! }
//! ```
//!
//! A full example can be seen at [examples/bank](https://github.com/thalo-rs/thalo/tree/main/examples/bank).

pub use actix::Message;
pub use app::*;
pub use async_trait::async_trait;
#[doc(inline)]
pub use command::*;
pub use error::Error;
#[doc(inline)]
pub use query::*;
pub use shared_global::*;
pub use thalo_macros::*;
pub use topic::*;

mod app;
mod command;
mod error;
pub mod postgres;
mod query;
mod shared_global;
mod topic;

/// Builds a thalo app.
pub fn new<ES>(event_store: ES, redpanda_host: impl Into<String> + Clone) -> App<ES>
where
    ES: EventStore + Clone + Send + Sync + Unpin + 'static,
{
    App::new(event_store, redpanda_host)
}
