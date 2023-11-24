use sled::{Batch, IVec, Tree};

use crate::error::Result;
use crate::stream::MessageIter;

#[derive(Clone)]
pub struct Outbox {
    tree: Tree,
}

impl Outbox {
    pub(crate) fn new(tree: Tree) -> Self {
        Outbox { tree }
    }

    pub fn iter_all_messages<T>(&self) -> MessageIter<T> {
        MessageIter::new(self.tree.iter())
    }

    pub fn delete_batch(&self, ids: Vec<IVec>) -> Result<()> {
        let mut batch = Batch::default();
        for id in ids {
            batch.remove(id);
        }

        Ok(self.tree.apply_batch(batch)?)
    }

    pub async fn flush_async(&self) -> Result<usize> {
        Ok(self.tree.flush_async().await?)
    }
}
