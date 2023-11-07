use std::borrow::Cow;
use std::future::Future;
use std::marker::PhantomData;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::SystemTime;
use std::{ops, str, task};

use serde::de::DeserializeOwned;
use serde::Deserialize;
use sled::{Db, Event, IVec, Tree};
use thalo::{Metadata, StreamName};
use tokio::sync::{Mutex, MutexGuard};
use tracing::info;

use crate::lock_registry::LockRegistry;
use crate::message::Message;
use crate::{Error, Result};

#[derive(Clone)]
pub struct MessageStore {
    pub db: Db,
    lock_registry: LockRegistry,
}

impl MessageStore {
    pub fn new(db: Db) -> Self {
        MessageStore {
            db,
            lock_registry: LockRegistry::new(),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(MessageStore::new(db))
    }

    pub fn stream<'a>(&self, stream_name: StreamName<'a>) -> Result<Stream<'a>> {
        let lock = self
            .lock_registry
            .get_category_lock(&stream_name.category());

        Ok(Stream {
            db: self.db.clone(),
            tree: self.db.open_tree(stream_name.as_bytes())?,
            stream_name,
            lock,
            version: None,
        })
    }
}

#[derive(Clone)]
pub struct Stream<'a> {
    pub db: Db,
    pub tree: Tree,
    stream_name: StreamName<'a>,
    lock: Arc<Mutex<()>>,
    version: Option<Option<u64>>,
}

#[derive(Clone)]
pub struct RawMessage<T: Clone> {
    ivec: IVec,
    marker: PhantomData<T>,
}

impl<T: Clone> RawMessage<T> {
    fn new(ivec: IVec) -> Self {
        RawMessage {
            ivec,
            marker: PhantomData,
        }
    }

    pub fn message<'a>(&'a self) -> Result<Message<'a, T>>
    where
        T: Deserialize<'a>,
    {
        serde_cbor::from_slice(&self.ivec).map_err(Error::DeserializeData)
    }
}

pub struct MessageIter<T> {
    inner: sled::Iter,
    marker: PhantomData<T>,
}

impl<T> MessageIter<T> {
    pub fn new(inner: sled::Iter) -> Self {
        MessageIter {
            inner,
            marker: PhantomData,
        }
    }
}

impl<T> Iterator for MessageIter<T>
where
    T: Clone,
{
    type Item = Result<RawMessage<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|res| res.map_err(Error::from).map(|(_, v)| RawMessage::new(v)))
    }
}

pub struct MessageSubscriber<T> {
    inner: sled::Subscriber,
    marker: PhantomData<T>,
}

impl<T> MessageSubscriber<T> {
    pub fn new(inner: sled::Subscriber) -> Self {
        MessageSubscriber {
            inner,
            marker: PhantomData,
        }
    }
}

impl<T> Future for MessageSubscriber<T>
where
    T: Clone,
{
    type Output = Option<RawMessage<T>>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        // SAFETY: This is safe because `sled::Subscriber` is Unpin.
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.inner) };
        match inner.poll(cx) {
            Poll::Ready(Some(Event::Insert { value, .. })) => {
                Poll::Ready(Some(RawMessage::new(value)))
            }
            Poll::Ready(Some(Event::Remove { .. })) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Stream<'_> {
    pub fn iter_all_messages<T>(&self) -> MessageIter<T>
    where
        T: Clone + DeserializeOwned + 'static,
    {
        MessageIter::new(self.tree.iter())
    }

    pub fn watch<T>(&self) -> MessageSubscriber<T>
    where
        T: Clone + DeserializeOwned + 'static,
    {
        MessageSubscriber::new(self.tree.watch_prefix(&[]))
    }

    pub async fn write_message(
        &mut self,
        msg_type: &str,
        data: &serde_json::Value,
        metadata: &Metadata<'_>,
        expected_version: Option<u64>,
    ) -> Result<u64> {
        let lock = self.lock.clone();
        let _guard = lock.lock().await;

        self.write_message_with_guard(&_guard, msg_type, data, metadata, expected_version)
    }

    pub async fn write_messages<'a, 'b, I>(
        &mut self,
        messages: I,
        expected_starting_version: Option<u64>,
    ) -> Result<u64>
    where
        'b: 'a,
        I: Iterator<Item = (&'a str, &'a serde_json::Value, &'a Metadata<'b>)> + 'a,
    {
        let lock = self.lock.clone();
        let _guard = lock.lock().await;

        let mut last_position = 0;

        for (i, (msg_type, data, metadata)) in messages.enumerate() {
            let expected_version = if i == 0 {
                expected_starting_version.map(|ev| ev + i as u64)
            } else {
                Some(
                    expected_starting_version
                        .map(|ev| ev + i as u64)
                        .unwrap_or(0),
                )
            };
            last_position =
                self.write_message_with_guard(&_guard, msg_type, data, metadata, expected_version)?;
        }

        Ok(last_position)
    }

    fn write_message_with_guard(
        &mut self,
        _guard: &MutexGuard<'_, ()>,
        msg_type: &str,
        data: &serde_json::Value,
        metadata: &Metadata<'_>,
        expected_version: Option<u64>,
    ) -> Result<u64> {
        let stream_version = self.version()?;

        if let Some(expected_version) = expected_version {
            if stream_version
                .map(|stream_version| expected_version != stream_version)
                .unwrap_or(true)
            {
                return Err(Error::WrongExpectedVersion {
                    expected_version,
                    stream_name: self.stream_name.to_string(),
                    stream_version,
                });
            }
        }

        let next_position = stream_version
            .map(|stream_version| stream_version + 1)
            .unwrap_or(0);

        let message = Message {
            id: self.db.generate_id()?,
            stream_name: self.stream_name.as_borrowed(),
            msg_type: Cow::Borrowed(msg_type),
            position: next_position,
            data: Cow::Borrowed(data),
            metadata: metadata.as_borrowed(),
            time: SystemTime::now(),
        };
        let raw_message = serde_cbor::to_vec(&message).map_err(Error::DeserializeData)?;
        self.tree
            .insert(self.db.generate_id()?.to_be_bytes(), raw_message)?;

        info!(id = message.id, stream_name = %message.stream_name, msg_type = %message.msg_type, position = message.position, data = ?message.data, "message written");

        self.version = Some(Some(next_position));

        Ok(next_position)
    }

    /// Returns the highest position number in the stream.
    pub fn version(&mut self) -> Result<Option<u64>> {
        match self.version {
            Some(version) => Ok(version),
            None => {
                let version = self.calculate_latest_version()?;
                self.version = Some(version);
                Ok(version)
            }
        }
    }

    fn calculate_latest_version(&self) -> Result<Option<u64>> {
        let mut max_position: Option<u64> = None;

        // Iterate over all messages to find the max position for the target stream name
        for result in self.iter() {
            let (_key, value) = result?;

            #[derive(Deserialize)]
            struct MessagePosition {
                position: u64,
            }

            // Deserialize the message
            let MessagePosition { position } =
                serde_cbor::from_slice(&value).map_err(Error::DeserializeData)?;

            max_position = Some(max_position.map_or(position, |max| max.max(position)));
        }

        Ok(max_position)
    }
}

impl ops::Deref for Stream<'_> {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}
