use std::ops;

use sled::{Db, Tree};

use crate::error::{Error, Result};
use crate::message::MessageData;
use crate::stream::RawMessage;

const GLOBAL_EVENT_LOG_TREE: &str = "thalo:global_event_log";

#[derive(Clone)]
pub struct GlobalEventLog {
    pub(crate) db: Db,
    tree: Tree,
}

impl GlobalEventLog {
    pub(crate) fn new(db: Db) -> Result<Self> {
        let tree = db.open_tree(GLOBAL_EVENT_LOG_TREE)?;
        Ok(GlobalEventLog { db, tree })
    }

    pub fn iter_all_messages(&self) -> GlobalEventLogIter {
        GlobalEventLogIter::new(self.db.clone(), self.tree.iter())
    }

    pub fn get(&self, id: u64) -> Result<Option<RawMessage<MessageData>>> {
        self.tree
            .get(id.to_be_bytes())?
            .map(|value| {
                let (stream_key, stream_name) = value.split_at(8);
                let tree = self.db.open_tree(stream_name)?;
                let message = tree.get(stream_key)?.ok_or_else(|| {
                    let stream_name = String::from_utf8_lossy(stream_name).into_owned();
                    Error::InvalidEventReference { id, stream_name }
                })?;

                Ok(RawMessage::new(id.to_be_bytes().to_vec().into(), message))
            })
            .transpose()
    }

    pub fn last_position(&self) -> Result<Option<u64>> {
        self.tree
            .last()?
            .map(|(k, _)| {
                let slice = k.as_ref().try_into().map_err(|_| Error::InvalidU64Id)?;
                Ok(u64::from_be_bytes(slice))
            })
            .transpose()
    }
}

impl ops::Deref for GlobalEventLog {
    type Target = Tree;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

pub struct GlobalEventLogIter {
    db: Db,
    inner: sled::Iter,
}

impl GlobalEventLogIter {
    fn new(db: Db, inner: sled::Iter) -> Self {
        GlobalEventLogIter { db, inner }
    }
}

impl Iterator for GlobalEventLogIter {
    type Item = Result<RawMessage<MessageData>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|res| {
            res.map_err(Error::from).and_then(|(global_id, id)| {
                let (id, stream_name) = id.split_at(8);
                let tree = self.db.open_tree(stream_name)?;
                let message = tree.get(id)?.ok_or_else(|| {
                    let id = id
                        .try_into()
                        .map(|id| u64::from_be_bytes(id))
                        .unwrap_or_default();
                    let stream_name = String::from_utf8_lossy(stream_name).into_owned();
                    Error::InvalidEventReference { id, stream_name }
                })?;

                Ok(RawMessage::new(global_id, message))
            })
        })
    }
}
