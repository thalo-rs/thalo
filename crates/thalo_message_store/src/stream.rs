use std::{borrow::Cow, marker::PhantomData, ops, time::SystemTime};

use serde::{de::DeserializeOwned, Deserialize};
use sled::{
    transaction::{ConflictableTransactionError, Transactional, TransactionalTree},
    IVec, Tree,
};
use thalo::StreamName;
use tracing::info;

use crate::{
    error::{Error, Result},
    global_event_log::GlobalEventLog,
    id_generator::IdGenerator,
    message::{GenericMessage, Message},
};

#[derive(Clone)]
pub struct Stream<'a> {
    id_generator: IdGenerator,
    tree: Tree,
    global_event_log: GlobalEventLog,
    stream_name: StreamName<'a>,
    version: Option<Option<u64>>,
}

impl<'a> Stream<'a> {
    pub(crate) fn new(
        id_generator: IdGenerator,
        tree: Tree,
        global_event_log: GlobalEventLog,
        stream_name: StreamName<'a>,
    ) -> Self {
        Stream {
            id_generator,
            tree,
            global_event_log,
            stream_name,
            version: None,
        }
    }

    pub fn stream_name(&self) -> &StreamName<'a> {
        &self.stream_name
    }

    pub fn iter_all_messages<T>(&self) -> MessageIter<T>
    where
        T: Clone + DeserializeOwned + 'static,
    {
        MessageIter::new(self.tree.iter())
    }

    pub fn write_messages<'b>(
        &'b mut self,
        messages: &[(&'b str, Cow<'b, serde_json::Value>)],
        expected_starting_version: Option<u64>,
    ) -> Result<Vec<GenericMessage<'b>>>
    where
        'a: 'b,
    {
        if messages.is_empty() {
            return Ok(vec![]);
        }

        let stream_version = self.version();

        let (written_messages, new_version) = (&self.tree, &*self.global_event_log).transaction(
            |(tx_stream, tx_global_event_log)| {
                let mut written_messages = Vec::with_capacity(messages.len());
                let mut stream_version = stream_version;

                for (i, (msg_type, data)) in messages.iter().enumerate() {
                    let expected_version = if i == 0 {
                        expected_starting_version.map(|ev| ev + i as u64)
                    } else {
                        Some(
                            expected_starting_version
                                .map(|ev| ev + i as u64)
                                .unwrap_or(0),
                        )
                    };
                    let global_id = self.id_generator.generate_id();
                    let written_message = Self::write_message_in_tx(
                        &tx_stream,
                        &tx_global_event_log,
                        global_id,
                        self.stream_name.as_borrowed(),
                        stream_version,
                        msg_type,
                        data.clone(),
                        expected_version,
                    )
                    .map_err(ConflictableTransactionError::Abort)?;
                    stream_version = Some(written_message.position);
                    written_messages.push(written_message);
                }

                tx_stream.flush();
                tx_global_event_log.flush();

                Ok((written_messages, stream_version))
            },
        )?;

        self.version = Some(new_version);

        Ok(written_messages)
    }

    fn write_message_in_tx<'b>(
        tx_stream: &TransactionalTree,
        tx_global_event_log: &TransactionalTree,
        global_id: u64,
        stream_name: StreamName<'b>,
        stream_version: Option<u64>,
        msg_type: &'b str,
        data: Cow<'b, serde_json::Value>,
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

        let message_id = tx_stream.generate_id()?;
        let message_id_bytes = message_id.to_be_bytes().to_vec();
        let mut message_ref = message_id_bytes.clone();
        message_ref.extend_from_slice(stream_name.as_bytes());
        let message = Message {
            id: message_id,
            global_id,
            position: next_position,
            stream_name,
            msg_type: Cow::Borrowed(msg_type),
            data,
            time: SystemTime::now(),
        };
        let raw_message = serde_cbor::to_vec(&message).map_err(|err| {
            ConflictableTransactionError::Abort(Box::new(Error::SerializeData(err)))
        })?;
        tx_stream.insert(message_id_bytes, raw_message.clone())?;
        tx_global_event_log.insert(global_id.to_be_bytes().to_vec(), message_ref)?;

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
pub struct RawMessage<T> {
    pub key: IVec,
    pub value: IVec,
    marker: PhantomData<T>,
}

impl<T> RawMessage<T> {
    pub(crate) fn new(key: IVec, value: IVec) -> Self {
        RawMessage {
            key,
            value,
            marker: PhantomData,
        }
    }

    pub fn id(&self) -> Result<u64> {
        let slice = self
            .key
            .as_ref()
            .try_into()
            .map_err(|_| Error::InvalidU64Id)?;
        Ok(u64::from_be_bytes(slice))
    }

    pub fn message<'a>(&'a self) -> Result<Message<'a, T>>
    where
        T: Clone + Deserialize<'a>,
    {
        serde_cbor::from_slice(&self.value).map_err(Error::DeserializeData)
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
            .map(|res| res.map_err(Error::from).map(|(k, v)| RawMessage::new(k, v)))
    }
}
