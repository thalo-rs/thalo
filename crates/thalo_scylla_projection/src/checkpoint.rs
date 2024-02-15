use std::sync::Arc;
use std::time::Duration;

use indicatif::{HumanCount, ProgressBar};
use scylla::prepared_statement::PreparedStatement;
use scylla::Session;
use tokio::sync::watch::{self, Sender};
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};
use tracing::{error, info};

#[derive(Clone)]
pub struct Checkpoint {
    session: Arc<Session>,
    id: String,
    select_stmt: PreparedStatement,
    update_stmt: PreparedStatement,
}

impl Checkpoint {
    pub async fn new(
        session: Arc<Session>,
        keyspace: &str,
        table_name: &str,
        id: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let select_stmt = session
            .prepare(format!(
                "SELECT last_global_sequence FROM {keyspace}.{table_name} WHERE id = ?"
            ))
            .await?;
        let update_stmt = session
            .prepare(format!(
                "INSERT INTO {keyspace}.{table_name} (id, last_global_sequence) VALUES (?, ?)"
            ))
            .await?;

        Ok(Checkpoint {
            session,
            id: id.into(),
            select_stmt,
            update_stmt,
        })
    }

    pub async fn get_last_global_sequence(&self) -> anyhow::Result<Option<u64>> {
        let last_global_sequence = self
            .session
            .execute(&self.select_stmt, (&self.id,))
            .await?
            .maybe_first_row_typed::<(i64,)>()?
            .map(|(n,)| n as u64);

        Ok(last_global_sequence)
    }

    pub async fn update_last_global_sequence(&self, global_sequence: u64) -> anyhow::Result<()> {
        self.session
            .execute(&self.update_stmt, (&self.id, global_sequence as i64))
            .await?;

        Ok(())
    }

    pub fn spawn_checkpoint_saver(
        &self,
        interval: Duration,
        pb: Option<ProgressBar>,
    ) -> (Sender<u64>, JoinHandle<()>) {
        let this = self.clone();
        let (tx, mut rx) = watch::channel(0);
        let handle = tokio::spawn(async move {
            let mut next_update_time = Instant::now();
            while let Ok(()) = rx.changed().await {
                if Instant::now() < next_update_time {
                    sleep_until(next_update_time).await;
                }

                let global_sequence = *rx.borrow_and_update();
                if let Err(err) = this.update_last_global_sequence(global_sequence).await {
                    error!("error updating last global sequence: {err}");
                }

                if let Some(pb) = &pb {
                    pb.set_prefix(HumanCount(global_sequence).to_string());
                }

                next_update_time = Instant::now() + interval;
            }
            info!("checkpoint saver stopping");
        });

        (tx, handle)
    }
}
