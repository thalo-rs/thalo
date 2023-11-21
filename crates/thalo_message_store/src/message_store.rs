use std::path::Path;

use sled::{Db, Mode};
use thalo::stream_name::{Category, StreamName};

use crate::error::Result;
use crate::global_event_log::GlobalEventLog;
use crate::id_generator::IdGenerator;
use crate::outbox::Outbox;
use crate::projection::{Projection, PROJECTION_POSITIONS_TREE};
use crate::stream::Stream;

#[derive(Clone)]
pub struct MessageStore {
    db: Db,
    id_generator: IdGenerator,
}

impl MessageStore {
    pub fn new(db: Db) -> Result<Self> {
        let global_event_log = GlobalEventLog::new(db)?;
        let last_id = global_event_log.last_position()?;
        let id_generator = IdGenerator::new(last_id);

        Ok(MessageStore {
            db: global_event_log.db,
            id_generator,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::Config::new()
            .flush_every_ms(None)
            .mode(Mode::LowSpace)
            .path(path)
            .open()?;
        MessageStore::new(db)
    }

    pub fn global_event_log(&self) -> Result<GlobalEventLog> {
        GlobalEventLog::new(self.db.clone())
    }

    pub fn stream<'a>(&self, stream_name: StreamName<'a>) -> Result<Stream<'a>> {
        Ok(Stream::new(
            self.id_generator.clone(),
            self.db.open_tree(stream_name.as_bytes())?,
            self.global_event_log()?,
            stream_name,
        ))
    }

    pub fn projection(&self, name: impl Into<String>) -> Result<Projection> {
        Projection::new(&self.db, name.into())
    }

    pub async fn flush_projections(&self) -> Result<usize> {
        let tree = self.db.open_tree(PROJECTION_POSITIONS_TREE)?;
        Ok(tree.flush_async().await?)
    }

    pub fn outbox(&self, category: Category<'_>) -> Result<Outbox> {
        let tree_name = Category::from_parts(category, &["outbox"])?;
        let tree = self.db.open_tree(tree_name.as_bytes())?;
        let outbox = Outbox::new(tree);
        Ok(outbox)
    }
}
