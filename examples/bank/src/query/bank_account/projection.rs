use async_trait::async_trait;
use awto_es::{
    postgres::PgEventStore,
    postgres::{tls::NoTls, PgRepository},
    Error, EventHandler, Projection,
};
use serde::{Deserialize, Serialize};

use crate::command::bank_account::BankAccountEvent;

use super::{BankAccountView, BankAccountViewRepository};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BankAccountProjectionEvent {
    BankAccount(BankAccountEvent),
}

#[derive(Clone)]
pub struct BankAccountProjector {
    event_store: PgEventStore<NoTls>,
    repository: BankAccountViewRepository,
}

impl BankAccountProjector {
    pub fn new(event_store: PgEventStore<NoTls>) -> Self {
        let repository = BankAccountViewRepository::new(event_store.pool().clone());

        Self {
            event_store,
            repository,
        }
    }
}

#[async_trait]
impl EventHandler for BankAccountProjector {
    type Event = BankAccountEvent;

    async fn handle(
        &mut self,
        id: String,
        event: BankAccountEvent,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error> {
        use BankAccountEvent::*;

        match event {
            AccountOpened { initial_balance } => {
                let view = BankAccountView {
                    account_number: id,
                    balance: initial_balance,
                };
                self.repository
                    .save(&view, event_id, event_sequence)
                    .await?;
            }
            FundsDeposited { amount } => {
                let mut view = self.repository.load(&id, event_id).await?;

                view.balance += amount;

                self.repository
                    .save(&view, event_id, event_sequence)
                    .await?;
            }
            FundsWithdrawn { amount } => {
                let mut view = self.repository.load(&id, event_id).await?;

                view.balance -= amount;

                self.repository
                    .save(&view, event_id, event_sequence)
                    .await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Projection for BankAccountProjector {
    fn projection_type() -> &'static str {
        "bank_account"
    }

    async fn last_event_id(&self) -> Result<Option<i64>, Error> {
        self.repository.last_event_id().await
    }
}
