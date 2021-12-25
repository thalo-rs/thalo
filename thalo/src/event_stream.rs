//! Event stream

#[cfg(feature = "with-tokio")]
use async_stream::try_stream;
use futures::stream::{BoxStream, StreamExt};
use serde::de::DeserializeOwned;

use crate::{aggregate::Aggregate, event::AggregateEventEnvelope};

// /// An alias to an active event stream returned by [`EventStream::listen_events`].
// pub type EventStreamIter<'a, A, E> = BoxStream<'a, Result<AggregateEventEnvelope<A>, E>>;

/// An event stream is a source of previously published events.
///
/// This is usually a consumer for message brokers.
pub trait EventStream<A: Aggregate> {
    /// Error yeilded in stream.
    type Error;

    /// Continuously listen for events as an async stream for the given aggregate.
    fn listen_events<'a>(
        &'a mut self,
    ) -> BoxStream<'a, Result<AggregateEventEnvelope<A>, Self::Error>>
    where
        <A as Aggregate>::Event: DeserializeOwned + Send + 'a;
}

#[cfg(feature = "with-tokio")]
impl<A> EventStream<A> for tokio::sync::broadcast::Receiver<AggregateEventEnvelope<A>>
where
    A: Aggregate,
    <A as Aggregate>::Event: Clone,
{
    type Error = tokio::sync::broadcast::error::RecvError;

    fn listen_events<'a>(
        &'a mut self,
    ) -> BoxStream<'a, Result<AggregateEventEnvelope<A>, Self::Error>>
    where
        <A as Aggregate>::Event: DeserializeOwned + Send + 'a,
    {
        (try_stream! {
            loop {
                let event = self.recv().await?;
                yield event;
            }
        })
        .boxed()
    }
}
