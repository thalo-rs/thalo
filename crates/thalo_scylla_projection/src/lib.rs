pub mod checkpoint;
mod event_processor;
pub mod events_stream;

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use std::{cmp, fmt};

use anyhow::Context;
use clap::Parser;
use futures::{Future, StreamExt};
use indicatif::{HumanCount, MultiProgress, ProgressBar, ProgressStyle};
use scylla::{Session, SessionBuilder};
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio::task::JoinHandle;
use tracing::{error, trace};

use crate::checkpoint::Checkpoint;
pub use crate::event_processor::*;
use crate::events_stream::EventsStream;

const DEFAULT_HOSTNAME: &str = "127.0.0.1";
const DEFAULT_EVENTS_TABLE: &str = "thalo.event_store";
const DEFAULT_EVENTS_BY_GLOBAL_SEQUENCE_TABLE: &str = "thalo.events_by_global_sequence";
const DEFAULT_CHECKPOINTS_TABLE: &str = "projection.checkpoints";
const DEFAULT_SLEEP_INTERVAL: f64 = 2.0;

#[derive(Parser)]
pub struct EventProcessorConfig {
    /// Address of a node in source cluster
    #[clap(short = 'H', long, env, default_value_t = DEFAULT_HOSTNAME.to_string())]
    pub hostname: String,

    /// Table name of the event store
    #[clap(long, env, default_value_t = DEFAULT_EVENTS_TABLE.to_string())]
    pub events_table: String,

    /// Name of the events by global sequence table
    #[clap(long, env, default_value_t = DEFAULT_EVENTS_BY_GLOBAL_SEQUENCE_TABLE.to_string())]
    pub events_by_global_sequence_table: String,

    /// Table name of the checkpoint
    #[clap(long, env, default_value_t = DEFAULT_CHECKPOINTS_TABLE.to_string())]
    pub checkpoints_table: String,

    /// Sleep interval in seconds between polling for new events
    #[clap(long, env, default_value_t = DEFAULT_SLEEP_INTERVAL)]
    pub sleep_interval: f64,

    #[clap(skip)]
    pub progress_bar: Option<ProgressBar>,
}

impl EventProcessorConfig {
    pub fn new() -> Self {
        EventProcessorConfig {
            hostname: DEFAULT_HOSTNAME.to_string(),
            events_table: DEFAULT_EVENTS_TABLE.to_string(),
            events_by_global_sequence_table: DEFAULT_EVENTS_BY_GLOBAL_SEQUENCE_TABLE.to_string(),
            checkpoints_table: DEFAULT_CHECKPOINTS_TABLE.to_string(),
            sleep_interval: DEFAULT_SLEEP_INTERVAL,
            progress_bar: None,
        }
    }

    pub async fn start<P, F, Fu>(
        self,
        id: impl Into<String>,
        event_processor: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(Arc<Session>, &Self) -> Fu,
        Fu: Future<Output = anyhow::Result<P>>,
        P: EventProcessor + Clone + Send + 'static,
        <P as EventProcessor>::Event: Send,
        <P as EventProcessor>::Error: fmt::Debug,
    {
        macro_rules! keyspace_table_name {
            ($k:expr) => {
                $k.split_once('.')
                    .with_context(|| format!("missing keyspace in {}", stringify!($k)))?
            };
        }

        let session = Arc::new(
            SessionBuilder::new()
                .known_node(&self.hostname)
                .build()
                .await
                .context("failed to connect to scylla")?,
        );

        let event_processor = event_processor(Arc::clone(&session), &self).await?;

        let events_stream = {
            let (keyspace, table_name) = keyspace_table_name!(self.events_by_global_sequence_table);
            EventsStream::new(Arc::clone(&session), keyspace, table_name).await?
        };

        let checkpoint = {
            let (keyspace, table_name) = keyspace_table_name!(self.checkpoints_table);
            Checkpoint::new(Arc::clone(&session), keyspace, table_name, id).await?
        };

        // Initialize progress bar values
        let last_global_sequence = checkpoint.get_last_global_sequence().await?;
        let mut next_global_sequence = last_global_sequence.map(|n| n + 1);
        let (checkpoint_tx, _) = checkpoint.spawn_checkpoint_saver(self.progress_bar.clone());
        if let Some(pb) = &self.progress_bar {
            pb.set_position(last_global_sequence.unwrap_or(0));
            pb.set_prefix(HumanCount(last_global_sequence.unwrap_or(0)).to_string());
        }

        // Continuously update the max global sequence every 10 seconds
        if let Some(pb) = self.progress_bar.clone() {
            let (keyspace, table_name) = keyspace_table_name!(self.events_by_global_sequence_table);
            Self::spawn_progress_bar_max_events_updater(
                Arc::clone(&session),
                keyspace,
                table_name,
                pb,
            )
            .await?;
        }

        // Update checkpoint
        let (status_tx, mut status_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut pending: BinaryHeap<Reverse<u64>> = BinaryHeap::new();
            let mut completed: BinaryHeap<Reverse<u64>> = BinaryHeap::new();
            let mut max_pending = 0;
            let mut max_completed = 0;

            while let Some(status) = status_rx.recv().await {
                match status {
                    EventStatus::Pending(global_sequence) => {
                        pending.push(Reverse(global_sequence));
                        max_pending = pending.len().max(max_pending);
                    }
                    EventStatus::Completed(global_sequence) => {
                        completed.push(Reverse(global_sequence));
                        max_completed = completed.len().max(max_completed);
                    }
                }

                if let Some(global_sequence) = lowest_completed(&mut pending, &mut completed) {
                    if let Some(pb) = &self.progress_bar {
                        pb.set_position(global_sequence);
                        if pb.length().unwrap_or(0) < global_sequence {
                            pb.set_length(global_sequence);
                        }
                    }
                    checkpoint_tx.send(global_sequence).unwrap();
                }
            }
        });

        // Iterate events and process them
        let mut locks: HashMap<String, Arc<Mutex<()>>> = HashMap::new();
        let semaphore = Arc::new(Semaphore::new(32));
        loop {
            let mut iter = events_stream
                .fetch_events(
                    next_global_sequence.map(|n| n / 1_000).unwrap_or(0),
                    next_global_sequence,
                )
                .await?;
            let mut processed_events_count = 0;
            while let Some(next_row_res) = iter.next().await {
                let ev = next_row_res?;
                let global_sequence = ev.global_sequence;
                let permit = semaphore.clone().acquire_owned().await?;
                let lock = locks.entry(ev.stream_name.clone()).or_default().clone();
                let guard = lock.lock_owned().await;
                status_tx.send(EventStatus::Pending(global_sequence))?;
                tokio::spawn({
                    let event_processor = event_processor.clone();
                    let status_tx = status_tx.clone();
                    async move {
                        event_processor.handle_event(ev).await.unwrap();
                        status_tx
                            .send(EventStatus::Completed(global_sequence))
                            .unwrap();
                        trace!(%global_sequence, "handled event");
                        std::mem::drop(guard);
                        std::mem::drop(permit);
                    }
                });
                next_global_sequence = Some(global_sequence + 1);
                processed_events_count += 1;
            }
            if processed_events_count == 0 {
                tokio::time::sleep(Duration::from_millis(
                    (self.sleep_interval / 1_000.0) as u64,
                ))
                .await;
            }
        }
    }

    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    pub fn events_table(mut self, events_table: impl Into<String>) -> Self {
        self.events_table = events_table.into();
        self
    }

    pub fn events_by_global_sequence_table(
        mut self,
        events_by_global_sequence_table: impl Into<String>,
    ) -> Self {
        self.events_by_global_sequence_table = events_by_global_sequence_table.into();
        self
    }

    pub fn checkpoints_table(mut self, checkpoints_table: impl Into<String>) -> Self {
        self.checkpoints_table = checkpoints_table.into();
        self
    }

    pub fn sleep_interval(mut self, sleep_interval: f64) -> Self {
        self.sleep_interval = sleep_interval;
        self
    }

    pub fn progress_bar(mut self, pb: impl Into<Option<ProgressBar>>) -> Self {
        self.progress_bar = pb.into();
        self
    }

    pub fn default_progress_bar(self) -> Self {
        self.progress_bar(default_progress_bar())
    }

    async fn spawn_progress_bar_max_events_updater(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
        progress_bar: ProgressBar,
    ) -> anyhow::Result<JoinHandle<()>> {
        let max_global_sequence_stmt = session
            .prepare(format!(
                "SELECT MAX(global_sequence) FROM {keyspace}.{table_name}"
            ))
            .await?;

        let handle = tokio::spawn({
            async move {
                loop {
                    let res = session
                        .execute(&max_global_sequence_stmt, ())
                        .await
                        .context("failed to query max global sequence")
                        .and_then(|res| {
                            res.first_row_typed::<(Option<i64>,)>()
                                .context("failed to get first row typed")
                        });
                    match res {
                        Ok((max_global_sequence,)) => {
                            progress_bar
                                .set_length(max_global_sequence.map(|max| max as u64).unwrap_or(0));
                        }
                        Err(err) => {
                            error!("{err}");
                        }
                    }

                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        });

        Ok(handle)
    }
}

pub fn default_progress_bar() -> ProgressBar {
    let m = MultiProgress::new();
    let sty = ProgressStyle::with_template(
        "[{elapsed_precise}] {percent:>3}% {bar:40.cyan/blue} {prefix:>10}/{human_pos}/{human_len:10} {msg}",
    )
    .unwrap()
    .progress_chars("##-");

    let pb = m.add(ProgressBar::new(0));
    pb.enable_steady_tick(Duration::from_millis(200));
    pb.set_style(sty.clone());
    pb.set_message("waiting for events...");

    pb
}

enum EventStatus {
    Pending(u64),
    Completed(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProcessingEventStatus {
    global_sequence: u64,
    done: bool,
}

impl PartialOrd for ProcessingEventStatus {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.global_sequence
            .partial_cmp(&other.global_sequence)
            .map(|ord| ord.reverse())
    }
}

impl Ord for ProcessingEventStatus {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.global_sequence.cmp(&other.global_sequence).reverse()
    }
}

fn lowest_completed(
    pending: &mut BinaryHeap<Reverse<u64>>,
    completed: &mut BinaryHeap<Reverse<u64>>,
) -> Option<u64> {
    let Reverse(pending_top) = pending.peek().copied()?;
    let mut last_completed_top = None;
    while let Some(Reverse(completed_top)) = completed.peek().copied() {
        if completed_top == pending_top {
            pending.pop();
            completed.pop();
            return Some(lowest_completed(pending, completed).unwrap_or(completed_top));
        }
        if completed_top > pending_top {
            break;
        }

        last_completed_top = completed.pop().map(|Reverse(n)| n);
    }

    last_completed_top
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_found() {
        let mut pending = BinaryHeap::new();
        let mut completed = BinaryHeap::new();
        pending.push(Reverse(1));
        pending.push(Reverse(3));
        completed.push(Reverse(2));
        completed.push(Reverse(1));

        let checkpoint = lowest_completed(&mut pending, &mut completed);
        assert_eq!(checkpoint, Some(2));
        assert_eq!(pending.peek(), Some(&Reverse(3)));
        assert_eq!(completed.peek(), None);
    }

    #[test]
    fn test_no_match() {
        let mut pending = BinaryHeap::new();
        let mut completed = BinaryHeap::new();
        pending.push(Reverse(3));
        pending.push(Reverse(4));
        completed.push(Reverse(1));
        completed.push(Reverse(2));

        let checkpoint = lowest_completed(&mut pending, &mut completed);
        assert_eq!(checkpoint, Some(2));
        assert_eq!(pending.peek(), Some(&Reverse(3)));
        assert!(completed.is_empty());
    }

    #[test]
    fn test_multiple_matches() {
        let mut pending = BinaryHeap::new();
        let mut completed = BinaryHeap::new();
        pending.push(Reverse(1));
        pending.push(Reverse(2));
        pending.push(Reverse(3));
        completed.push(Reverse(3));
        completed.push(Reverse(1));

        let checkpoint = lowest_completed(&mut pending, &mut completed);
        assert_eq!(checkpoint, Some(1));
        assert_eq!(pending.peek(), Some(&Reverse(2)));
        assert_eq!(completed.peek(), Some(&Reverse(3)));
    }

    #[test]
    fn test_empty_collections() {
        let mut pending = BinaryHeap::new();
        let mut completed = BinaryHeap::new();

        let checkpoint = lowest_completed(&mut pending, &mut completed);
        assert_eq!(checkpoint, None);
        assert!(pending.is_empty());
        assert!(completed.is_empty());
    }
}
