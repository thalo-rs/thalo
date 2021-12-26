//! Event stream

use futures_util::stream::{BoxStream, StreamExt};
use serde::de::DeserializeOwned;

use crate::{aggregate::Aggregate, event::AggregateEventEnvelope, Infallible};

/// An event stream is a source of previously published events.
///
/// This is usually a consumer for message brokers.
pub trait EventStream<A: Aggregate> {
    /// Error yeilded in stream.
    type Error;

    /// Continuously listen for events as an async stream for the given aggregate.
    fn listen_events(&mut self) -> BoxStream<'_, Result<AggregateEventEnvelope<A>, Self::Error>>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send;
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::ReceiverStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Error = Infallible;

    fn listen_events(&mut self) -> BoxStream<'_, Result<AggregateEventEnvelope<A>, Self::Error>>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        self.map(Ok).boxed()
    }
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::BroadcastStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Error = tokio_stream::wrappers::errors::BroadcastStreamRecvError;

    fn listen_events(&mut self) -> BoxStream<'_, Result<AggregateEventEnvelope<A>, Self::Error>>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        self.boxed()
    }
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::WatchStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Error = Infallible;

    fn listen_events(&mut self) -> BoxStream<'_, Result<AggregateEventEnvelope<A>, Self::Error>>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        self.map(Ok).boxed()
    }
}
