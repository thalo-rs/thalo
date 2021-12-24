//! Thalo v2

#![deny(missing_docs)]

/// Aggregates
pub mod aggregate;

/// Events
pub mod event;

/// Event store
#[cfg(feature = "event-store")]
pub mod event_store;

/// Tests config
#[cfg(feature = "tests-cfg")]
pub mod tests_cfg;

/// Unique type identifier
pub trait TypeId {
    /// Returns a unique identifier for the given type
    fn type_id() -> &'static str;
}
