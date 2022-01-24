//! A framework for building large-scale event sourced microservices.
//!
//! Thalo is a framework for creating event driven apps with event sourcing.
//! Some closely related patterns are used, including [event sourcing](https://microservices.io/patterns/data/event-sourcing.html), [CQRS](https://microservices.io/patterns/data/cqrs.html), [transactional outbox](https://microservices.io/patterns/data/transactional-outbox.html), [event driven](https://martinfowler.com/articles/201701-event-driven.html), [DDD](https://martinfowler.com/bliki/DomainDrivenDesign.html).
//!
//! **Core Modules**
//!
//! - [Aggregates](crate::aggregate) - Consistency boundary around a domain entity responsible for handling commands and applying events.
//! - [Events](crate::event) - Events that occured in your system.
//! - [Event Store](crate::event_store) - Event store containing all application events.
//! - [Event Stream](crate::event_stream) - Event stream, such as Kafka/RedPanda.
//!
//! **Official Crates**
//!
//! Core
//!
//! - [thalo](https://github.com/thalo-rs/thalo) - Core framework (this crate).
//! - [thalo-testing](https://docs.rs/thalo-testing) - Test utils for thalo apps.
//! - [thalo-macros](https://docs.rs/thalo-macros) - Macros for implementing traits. This can be enabled in the core crate with the `macros` feature flag.
//!
//! Event stores
//!
//! - [thalo-postgres](https://docs.rs/thalo-postgres) - Postgres implementation of [`EventStore`](crate::event_store::EventStore).
//! - [thalo-inmemory](https://docs.rs/thalo-inmemory) - In-memory implementation of [`EventStore`](crate::event_store::EventStore).
//! - [thalo-filestore](https://docs.rs/thalo-filestore) - Filestore implementation of [`EventStore`](crate::event_store::EventStore).
//!
//! Event streams
//!
//! - [thalo-kafka](https://docs.rs/thalo-kafka) - Kafka implementation of [`EventStream`](crate::event_stream::EventStream).

//!
//! ## Current status
//!
//! Thalo is still under heavy development and is not production ready.
//!
//! ## Get in touch
//!
//! If you'd like to ask/discuss or learn more, you can reach via the liks below:
//!
//! - [Github](https://github.com/tqwewe)
//! - [Discord Server](https://discord.gg/4Cq8NnPYPA)
//!
//! ## Examples
//!
//! Examples can be seen in the [`examples`](https://github.com/thalo-rs/thalo/tree/main/examples) directory.

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod aggregate;

pub mod event;

#[cfg(feature = "event-store")]
pub mod event_store;

#[cfg(feature = "event-stream")]
pub mod event_stream;

#[doc(hidden)]
#[cfg(any(test, doctest, feature = "tests-cfg"))]
pub mod tests_cfg;

/// An infallible error type typically used for event handlers or event streams that do not fail.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Infallible;

/// Include generated aggregate code.
///
/// You must specify the aggregate name as defined in the schema:
///
/// ```rust,ignore
/// include_aggregate!("BankAccount");
/// ```
///
/// # Note
///
/// **This only works when used with [esdl](https://docs.rs/esdl) in a build script.**
/// To learn more about thalo schemas, see the [esdl](https://github.com/thalo-rs/esdl) repository.
#[macro_export]
macro_rules! include_aggregate {
    ($name: tt) => {
        include!(concat!(env!("OUT_DIR"), concat!("/", $name, ".rs")));
    };
}
