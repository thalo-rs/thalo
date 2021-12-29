//! Kafka event stream for consuming events for [Thalo](https://docs.rs/thalo) apps.
//!
//! For a quickstart on listening to events with [`thalo::event::EventHandler`]'s, see  the [`watch_event_handlers`] macro.

#![deny(missing_docs)]

pub use config::KafkaClientConfig;
pub use error::Error;
pub use event_handler::WatchEventHandler;
pub use event_stream::{KafkaEventMessage, KafkaEventStream};
pub use topic::Topic;

mod config;
mod error;
mod event_handler;
mod event_stream;
mod topic;
