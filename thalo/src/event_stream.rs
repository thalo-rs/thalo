//! Event stream

use futures_util::stream::{BoxStream, StreamExt};
use serde::de::DeserializeOwned;

use crate::{aggregate::Aggregate, event::AggregateEventEnvelope, Infallible};

/// A type alias for the return type of [`EventStream::listen_events`].
pub type EventStreamResult<'a, I, E> = Result<BoxStream<'a, I>, E>;

/// An event stream is a source of previously published events.
///
/// This is usually a consumer for message brokers.
pub trait EventStream<A: Aggregate> {
    /// Item yielded by stream.
    type Item;

    /// Error before entering the stream.
    type Error;

    /// Continuously listen for events as an async stream for the given aggregate.
    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send;
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::ReceiverStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Item = AggregateEventEnvelope<A>;
    type Error = Infallible;

    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        Ok(self.boxed())
    }
}

#[cfg(feature = "with-tokio-stream")]
impl<A> EventStream<A> for tokio_stream::wrappers::BroadcastStream<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Item =
        Result<AggregateEventEnvelope<A>, tokio_stream::wrappers::errors::BroadcastStreamRecvError>;
    type Error = Infallible;

    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
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
    type Item = AggregateEventEnvelope<A>;
    type Error = Infallible;

    fn listen_events(&mut self) -> EventStreamResult<'_, Self::Item, Self::Error>
    where
        <A as Aggregate>::Event: 'static + DeserializeOwned + Send,
    {
        Ok(self.boxed())
    }
}
