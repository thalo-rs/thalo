//! Thalo Postgres

#![deny(missing_docs)]

pub use bb8_postgres::tokio_postgres::tls::*;
pub use error::Error;
pub use event_store::PgEventStore;

mod error;
mod event_store;
