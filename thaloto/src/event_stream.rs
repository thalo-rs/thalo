//! Event stream

use futures_core::stream::BoxStream;
use serde::de::DeserializeOwned;

use crate::aggregate::Aggregate;

/// An event stream is a source of previously published events.
///
/// This is usually a consumer for message brokers.
pub trait EventStream<A: Aggregate> {
    /// Async stream.
    type StreamOutput;

    /// Continuously listen for events as an async stream for the given aggregate.
    fn listen_events<'a>(&'a self) -> BoxStream<'a, Self::StreamOutput>
    where
        <A as Aggregate>::Event: DeserializeOwned + Send + 'a;
}
