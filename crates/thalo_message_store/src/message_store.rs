use std::borrow::Cow;
use std::path::Path;

use async_trait::async_trait;
use futures::stream::BoxStream;
use sled::{Db, Mode};
use thalo::event_store::message::Message;
use thalo::event_store::{EventStore, NewEvent};
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

#[async_trait]
impl EventStore for MessageStore {
    async fn iter_stream(
        &self,
        stream_name: &str,
    ) -> anyhow::Result<BoxStream<'_, anyhow::Result<Message<'static>>>> {
        let iter = self
            .stream(StreamName::new(stream_name)?)?
            .iter_all_messages::<serde_json::Value>()
            .map(|res| {
                let raw_message = res?;
                let message = raw_message.message()?.into_owned();
                Result::<_, anyhow::Error>::Ok(message)
            });
        Ok(Box::pin(futures::stream::iter(iter)))
    }

    async fn append_to_stream(
        &self,
        stream_name: &str,
        expected_version: ExpectedVersion,
        events: Vec<NewEvent>,
    ) -> anyhow::Result<Vec<Message<'static>>> {
        let messages: Vec<_> = events
            .iter()
            .map(|event| (event.event_type.as_str(), Cow::Borrowed(&event.data)))
            .collect();
        let mut stream = self.stream(StreamName::new(stream_name)?)?;
        let expected_starting_version = match expected_version {
            ExpectedVersion::Any => None,
            ExpectedVersion::StreamExists => None,
            ExpectedVersion::NoStream => None,
            ExpectedVersion::Exact(v) => Some(v),
        };
        let written_messages = stream
            .write_messages(&messages, expected_starting_version)?
            .into_iter()
            .map(|message| message.into_owned())
            .collect();

        Ok(written_messages)
    }

    async fn latest_global_version(&self) -> anyhow::Result<Option<u64>> {
        Ok(self.global_event_log()?.last_position()?)
    }
}
