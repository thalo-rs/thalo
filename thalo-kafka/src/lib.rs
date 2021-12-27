//! Thalo Kafka

#![deny(missing_docs)]

pub use config::KafkaClientConfig;
pub use error::Error;
pub use event_stream::KafkaEventStream;

mod config;
mod error;
mod event_stream;
