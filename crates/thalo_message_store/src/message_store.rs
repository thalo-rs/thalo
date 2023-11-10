use std::borrow::Cow;
use std::future::Future;
use std::marker::PhantomData;
use std::path::Path;
use std::pin::Pin;
use std::task::Poll;
use std::time::SystemTime;
use std::{ops, str, task};

use serde::de::DeserializeOwned;
use serde::Deserialize;
use sled::transaction::{ConflictableTransactionError, TransactionError, TransactionalTree};
use sled::{Db, Event, IVec, Mode, Tree};
use thalo::{Metadata, StreamName};
use tracing::info;

use crate::message::{GenericMessage, Message};
use crate::{Error, Result};

#[derive(Clone)]
pub struct MessageStore {
    db: Db,
}

impl MessageStore {
    pub fn new(db: Db) -> Self {
        MessageStore { db }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::Config::new()
            .flush_every_ms(None)
            .mode(Mode::LowSpace)
            .path(path)
            .open()?;
        Ok(MessageStore::new(db))
    }

    pub fn stream<'a>(&self, stream_name: StreamName<'a>) -> Result<Stream<'a>> {
        Ok(Stream {
            db: self.db.clone(),
            tree: self.db.open_tree(stream_name.as_bytes())?,
            stream_name,
            version: None,
        })
    }
}

#[derive(Clone)]
pub struct Stream<'a> {
    db: Db,
    tree: Tree,
    stream_name: StreamName<'a>,
    version: Option<Option<u64>>,
}

impl<'a> Stream<'a> {
    pub fn stream_name(&self) -> &StreamName<'a> {
        &self.stream_name
    }

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

    pub async fn write_message<'b>(
        &'b mut self,
        msg_type: &'b str,
        data: Cow<'b, serde_json::Value>,
        metadata: Metadata<'b>,
        expected_version: Option<u64>,
    ) -> Result<GenericMessage<'b>>
    where
        'a: 'b,
    {
        let stream_version = self.version();
        let message = Self::write_message_with_guard(
            &DbTreeOrTransaction::DbTree(&self.db, &self.tree),
            self.stream_name.as_borrowed(),
            stream_version,
            msg_type,
            data,
            metadata,
            expected_version,
        )
        .map_err(|err| Error::DatabaseTransaction(TransactionError::Abort(err)))?;
        self.version = Some(Some(message.position));

        self.db.flush_async().await?;

        Ok(message)
    }

    pub async fn write_messages<'b>(
        &'b mut self,
        messages: &[(&'b str, Cow<'b, serde_json::Value>, Metadata<'b>)],
        expected_starting_version: Option<u64>,
    ) -> Result<Vec<GenericMessage<'b>>>
    where
        'a: 'b,
    {
        match messages.len() {
            0 => Ok(vec![]),
            1 => {
                let (msg_type, data, metadata) = &messages[0];
                self.write_message(
                    msg_type,
                    data.clone(),
                    metadata.clone(),
                    expected_starting_version,
                )
                .await
                .map(|msg| vec![msg])
            }
            _ => {
                let stream_version = self.version();

                let (written_messages, new_version) = self.tree.transaction(|tx| {
                    let mut written_messages = Vec::with_capacity(messages.len());
                    let mut stream_version = stream_version;

                    let tx = DbTreeOrTransaction::Transaction(&tx);
                    for (i, (msg_type, data, metadata)) in messages.iter().enumerate() {
                        let expected_version = if i == 0 {
                            expected_starting_version.map(|ev| ev + i as u64)
                        } else {
                            Some(
                                expected_starting_version
                                    .map(|ev| ev + i as u64)
                                    .unwrap_or(0),
                            )
                        };
                        let written_message = Self::write_message_with_guard(
                            &tx,
                            self.stream_name.as_borrowed(),
                            stream_version,
                            msg_type,
                            data.clone(),
                            metadata.clone(),
                            expected_version,
                        )
                        .map_err(ConflictableTransactionError::Abort)?;
                        stream_version = Some(written_message.position);
                        written_messages.push(written_message);
                    }

                    Ok((written_messages, stream_version))
                })?;

                self.version = Some(new_version);

                self.db.flush_async().await?;

                Ok(written_messages)
            }
        }
    }

    fn write_message_with_guard<'b>(
        tx: &DbTreeOrTransaction<'_>,
        stream_name: StreamName<'b>,
        stream_version: Option<u64>,
        msg_type: &'b str,
        data: Cow<'b, serde_json::Value>,
        metadata: Metadata<'b>,
        expected_version: Option<u64>,
    ) -> Result<GenericMessage<'b>, ConflictableTransactionError<Box<Error>>> {
        if let Some(expected_version) = expected_version {
            if stream_version
                .map(|stream_version| expected_version != stream_version)
                .unwrap_or(true)
            {
                return Err(ConflictableTransactionError::Abort(Box::new(
                    Error::WrongExpectedVersion {
                        expected_version,
                        stream_name: stream_name.to_string(),
                        stream_version,
                    },
                )));
            }
        }

        let next_position = stream_version
            .map(|stream_version| stream_version + 1)
            .unwrap_or(0);

        let message = Message {
            id: tx.generate_id()?,
            stream_name,
            msg_type: Cow::Borrowed(msg_type),
            position: next_position,
            data,
            metadata,
            time: SystemTime::now(),
        };
        let raw_message = serde_cbor::to_vec(&message).map_err(|err| {
            ConflictableTransactionError::Abort(Box::new(Error::DeserializeData(err)))
        })?;
        tx.insert(message.id.to_be_bytes().to_vec(), raw_message)?;

        info!(id = message.id, stream_name = %message.stream_name, msg_type = %message.msg_type, position = message.position, data = ?message.data, "message written");

        Ok(message)
    }

    /// Returns the highest position number in the stream.
    pub fn version(&mut self) -> Option<u64> {
        match self.version {
            Some(version) => version,
            None => {
                let version = self.calculate_latest_version();
                self.version = Some(version);
                version
            }
        }
    }

    fn calculate_latest_version(&self) -> Option<u64> {
        match self.len() {
            0 => None,
            i => Some(i as u64 - 1),
        }
    }
}

impl ops::Deref for Stream<'_> {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
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

enum DbTreeOrTransaction<'a> {
    DbTree(&'a Db, &'a Tree),
    Transaction(&'a TransactionalTree),
}

impl DbTreeOrTransaction<'_> {
    fn generate_id(&self) -> Result<u64, ConflictableTransactionError<Box<Error>>> {
        match self {
            DbTreeOrTransaction::DbTree(db, _) => Ok(db.generate_id()?),
            DbTreeOrTransaction::Transaction(tx) => Ok(tx.generate_id()?),
        }
    }

    fn insert<K, V>(
        &self,
        key: K,
        value: V,
    ) -> Result<Option<IVec>, ConflictableTransactionError<Box<Error>>>
    where
        K: AsRef<[u8]> + Into<IVec>,
        V: Into<IVec>,
    {
        match self {
            DbTreeOrTransaction::DbTree(_, tree) => tree
                .insert(key, value)
                .map_err(|err| ConflictableTransactionError::Abort(Box::new(err.into()))),
            DbTreeOrTransaction::Transaction(tx) => Ok(tx.insert(key, value)?),
        }
    }
}