use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use scylla::{FromRow, Session};
use scylla_cdc::checkpoints::CDCCheckpointSaver;
use scylla_cdc::consumer::{Consumer, ConsumerFactory};
use scylla_cdc::log_reader::CDCLogReaderBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[async_trait]
pub trait EventHandler: Send {
    async fn handle_event(&self, event: RecordedEvent) -> anyhow::Result<()>;
}

#[async_trait]
pub trait EventHandlerFactory: Sync + Send {
    async fn new_event_handler(&self) -> Box<dyn EventHandler>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEvent {
    pub stream_name: String,
    pub sequence: u64,
    pub id: Uuid,
    pub event_type: String,
    pub data: Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(FromRow)]
struct RawRecordedEvent {
    stream_name: String,
    sequence: i64,
    id: Uuid,
    event_type: String,
    data: Vec<u8>,
    timestamp: DateTime<Utc>,
    #[allow(unused)]
    date: NaiveDate,
}

impl TryFrom<RawRecordedEvent> for RecordedEvent {
    type Error = serde_json::Error;

    fn try_from(ev: RawRecordedEvent) -> Result<Self, Self::Error> {
        debug_assert_eq!(ev.timestamp.date_naive(), ev.date);

        Ok(RecordedEvent {
            stream_name: ev.stream_name,
            sequence: ev.sequence as u64,
            id: ev.id,
            event_type: ev.event_type,
            data: serde_json::from_slice(&ev.data)?,
            timestamp: ev.timestamp,
        })
    }
}

pub struct EventHandlerBuilder {
    checkpoint_saver: Option<Arc<dyn CDCCheckpointSaver>>,
    event_handler_factory: Option<Arc<dyn EventHandlerFactory>>,
    keyspace: Option<String>,
    safety_interval: Duration,
    save_interval: Duration,
    session: Option<Arc<Session>>,
    sleep_interval: Duration,
    table_name: Option<String>,
}

impl EventHandlerBuilder {
    pub fn new() -> Self {
        EventHandlerBuilder {
            checkpoint_saver: None,
            event_handler_factory: None,
            keyspace: None,
            safety_interval: Duration::from_secs(30),
            save_interval: Duration::from_secs(10),
            session: None,
            sleep_interval: Duration::from_secs(10),
            table_name: None,
        }
    }

    pub fn checkpoint_saver(mut self, cp_saver: Arc<dyn CDCCheckpointSaver>) -> Self {
        self.checkpoint_saver = Some(cp_saver);
        self
    }

    pub fn event_handler_factory(
        mut self,
        event_handler_factory: Arc<dyn EventHandlerFactory>,
    ) -> Self {
        self.event_handler_factory = Some(event_handler_factory);
        self
    }

    pub fn keyspace(mut self, keyspace: impl Into<String>) -> Self {
        self.keyspace = Some(keyspace.into());
        self
    }

    pub fn safety_interval(mut self, safety_interval: Duration) -> Self {
        self.safety_interval = safety_interval;
        self
    }

    pub fn save_interval(mut self, save_interval: Duration) -> Self {
        self.save_interval = save_interval;
        self
    }

    pub fn session(mut self, session: Arc<Session>) -> Self {
        self.session = Some(session);
        self
    }

    pub fn sleep_interval(mut self, sleep_interval: Duration) -> Self {
        self.sleep_interval = sleep_interval;
        self
    }

    pub fn table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }

    pub async fn build(self) -> anyhow::Result<EventHandlerRuntime> {
        let mut cdc = CDCLogReaderBuilder::new()
            .consumer_factory(Arc::new(EventHandlerCDCConsumerFactory {}))
            .safety_interval(Duration::ZERO)
            .sleep_interval(self.sleep_interval)
            .start_timestamp(Utc::now() - self.safety_interval)
            .window_size(self.sleep_interval.max(self.safety_interval));

        if let Some(checkpoint_saver) = self.checkpoint_saver {
            cdc = cdc
                .checkpoint_saver(checkpoint_saver)
                .should_load_progress(true)
                .should_save_progress(false);
        }

        if let Some(keyspace) = &self.keyspace {
            cdc = cdc.keyspace(keyspace);
        }

        if let Some(session) = self.session {
            cdc = cdc.session(session);
        }

        if let Some(table_name) = &self.table_name {
            cdc = cdc.table_name(table_name);
        }

        let (cdc, handle) = cdc.build().await?;

        todo!()
    }
}

pub struct EventHandlerRuntime {}

struct EventHandlerCDCConsumerFactory {}

#[async_trait]
impl ConsumerFactory for EventHandlerCDCConsumerFactory {
    async fn new_consumer(&self) -> Box<dyn Consumer> {
        todo!()
    }
}

pub async fn start_event_handler(
    handler: impl EventHandler,
    opts: EventHandlerOpts,
) -> anyhow::Result<()> {
    // Handle old events
}
