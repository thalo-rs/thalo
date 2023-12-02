use async_trait::async_trait;
use counter::Incremented;
use serde::Deserialize;
use sled::transaction::TransactionError;
use sled::Db;
use thalo_runtime::{Message, Projection};

const LAST_GLOBAL_ID_KEY: [u8; 1] = [0];
const COUNT_KEY: [u8; 1] = [1];

pub struct CountProjection {
    db: Db,
    count: u64,
}

impl CountProjection {
    pub fn new(db: Db) -> anyhow::Result<Self> {
        let count = db
            .get(COUNT_KEY)?
            .map(|value| {
                let slice = value.as_ref().try_into().unwrap();
                u64::from_be_bytes(slice)
            })
            .unwrap_or(0);

        Ok(CountProjection { db, count })
    }
}

#[derive(Deserialize)]
pub enum Event {
    Incremented(Incremented),
}

#[async_trait]
impl Projection for CountProjection {
    type Event = Event;

    async fn handle(&mut self, message: Message<'static, Event>) -> anyhow::Result<()> {
        match message.event()? {
            Event::Incremented(Incremented { amount }) => {
                self.count += amount;
            }
        }

        self.db
            .transaction(|tx| {
                // Save the count
                tx.insert(COUNT_KEY.to_vec(), self.count.to_be_bytes().to_vec())?;

                // Update last global ID
                tx.insert(
                    LAST_GLOBAL_ID_KEY.to_vec(),
                    message.global_id.to_be_bytes().to_vec(),
                )?;

                // Flush the database
                tx.flush();

                Ok(())
            })
            .map_err(|err: TransactionError<anyhow::Error>| match err {
                TransactionError::Abort(err) => err,
                TransactionError::Storage(err) => err.into(),
            })?;

        println!("Count: {}", self.count);

        Ok(())
    }

    async fn last_global_id(&self) -> anyhow::Result<Option<u64>> {
        Ok(self
            .db
            .get(LAST_GLOBAL_ID_KEY)?
            .map(|v| u64::from_be_bytes(v.as_ref().try_into().unwrap())))
    }
}
