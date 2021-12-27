//! Event stream

use futures_util::stream::{BoxStream, StreamExt};
use serde::de::DeserializeOwned;

use crate::{aggregate::Aggregate, event::AggregateEventEnvelope, Infallible};

/// A type alias for the return type of [`EventStream::listen_events`].
pub type EventStreamResult<'a, A, E, SE> =
    Result<BoxStream<'a, Result<AggregateEventEnvelope<A>, SE>>, E>;

/// An event stream is a source of previously published events.
///
/// This is usually a consumer for message brokers.
pub trait EventStream<A: Aggregate> {
    /// Error before entering the stream.
    type Error;

    /// Error yeilded in stream.
    type StreamError;

    /// Continuously listen for events as an async stream for the given aggregate.
    fn listen_events(&mut self) -> EventStreamResult<'_, A, Self::Error, Self::StreamError>
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
    type StreamError = Infallible;

    fn listen_events(&mut self) -> EventStreamResult<'_, A, Self::Error, Self::StreamError>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        Ok(self.map(Ok).boxed())
    }
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::BroadcastStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Error = Infallible;
    type StreamError = tokio_stream::wrappers::errors::BroadcastStreamRecvError;

    fn listen_events(&mut self) -> EventStreamResult<'_, A, Self::Error, Self::StreamError>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        Ok(self.boxed())
    }
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::WatchStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Error = Infallible;
    type StreamError = Infallible;

    fn listen_events(&mut self) -> EventStreamResult<'_, A, Self::Error, Self::StreamError>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        Ok(self.map(Ok).boxed())
    }
}
