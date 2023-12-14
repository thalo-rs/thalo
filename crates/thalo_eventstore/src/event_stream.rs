use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use eventstore::ReadStream;
use futures::future::BoxFuture;
use futures::lock::Mutex;
use futures::{FutureExt, Stream};

use crate::RecordedEvent;

pub struct EventStream {
    read_stream: Option<Arc<Mutex<ReadStream>>>,
    next_future:
        Option<BoxFuture<'static, Result<Option<eventstore::ResolvedEvent>, eventstore::Error>>>,
}

impl EventStream {
    pub(crate) fn new(read_stream: Option<ReadStream>) -> Self {
        EventStream {
            read_stream: read_stream.map(|read_stream| Arc::new(Mutex::new(read_stream))),
            next_future: None,
        }
    }
}

impl Stream for EventStream {
    type Item = Result<RecordedEvent, eventstore::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(read_stream) = &self.read_stream else {
            return Poll::Ready(None);
        };

        if self.next_future.is_none() {
            let read_stream = read_stream.clone();

            let future = async move {
                let mut read_stream = read_stream.lock().await;
                match read_stream.next().await {
                    Ok(next) => Ok(next),
                    Err(eventstore::Error::ResourceNotFound) => Ok(None),
                    Err(err) => Err(err),
                }
            }
            .boxed();

            self.next_future = Some(future);
        }

        let fut = self.next_future.as_mut().unwrap();
        match fut.as_mut().poll(cx) {
            Poll::Ready(result) => {
                self.next_future.take(); // Clear the future so it's re-created next call
                Poll::Ready(
                    result
                        .map(|event| {
                            event.map(|event| match event.link {
                                Some(event) => RecordedEvent(event),
                                None => RecordedEvent(
                                    event
                                        .event
                                        .expect("if no link, then event should be present"),
                                ),
                            })
                        })
                        .transpose(),
                )
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
