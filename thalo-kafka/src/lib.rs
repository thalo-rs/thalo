//! Kafka event stream for consuming events for [Thalo](https://docs.rs/thalo) apps.
//!
//! For more information on watching events with Kafka, see [`spawn_event_handlers`].
//!
//! # Example
//!
//! ```
//! use thalo_kafka::spawn_event_handlers;
//!
//! use crate::projections::{BankAccountProjection, AssetsProjection};
//!
//! mod projections;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     tracing_subscriber::fmt().init();
//!
//!     let kafka_host = std::env::var("KAFKA_HOST").expect("missing kafka_host env var");
//!     let database_url = std::env::var("DATABASE_URL").expect("missing database_url env var");
//!     
//!     let db = Database::connect(&database_url).await?;
//!
//!     let handles = spawn_event_handlers!(
//!         kafka_host,
//!         ("bank-account", BankAccountProjection::new(db.clone())),
//!         ("assets", AssetsProjection::new(db)),
//!     );
//!     
//!     for handle in handles {
//!         handle.await??;
//!     }
//!
//!     Ok(())
//! }
//! ```

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

/// Spawn multiple event handlers in separate tokio tasks and returns a vector of the join handles.
///
/// Each event handler must implement [`WatchEventHandler`].
/// The first parameter is the kafka brokers string.
/// Following parameters are a tuple of 2 items, the first being the group_id,
/// and the second being an instance of a type that implements [`WatchEventHandler`].
///
/// Kafka will send messages to one consumer given the same [group id](https://kafka.apache.org/documentation/#consumerconfigs_group.id).
/// For this reason, each event handler should have a unique group id specific.
///
/// # Example
///
/// ```
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let kafka_host = std::env::var("KAFKA_HOST").expect("missing kafka_host env var");
///     let database_url = std::env::var("DATABASE_URL").expect("missing database_url env var");
///     
///     let db = Database::connect(&database_url).await?;
///
///     let handles = spawn_event_handlers!(
///         kafka_host,
///         ("bank-account", BankAccountProjection::new(db.clone())),
///         ("assets", AssetsProjection::new(db)),
///     );
///     
///     for handle in handles {
///         handle.await??;
///     }
///
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! spawn_event_handlers {
    ($brokers: expr, $( ( $group_id: expr, $event_handler: expr ) ),* $(,)?) => {
        vec![
            $({
                let event_handler = $event_handler;
                tokio::spawn(async move {
                    thalo_kafka::WatchEventHandler::watch(&event_handler, &$brokers, $group_id).await
                })
            }),*
        ]
    };
}
