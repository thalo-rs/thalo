use serde::{Deserialize, Serialize};
use thalo::{
    async_trait,
    postgres::PgEventStore,
    postgres::{tokio_postgres::tls::NoTls, PgRepository},
    Error, EventHandler, Projection,
};

use crate::command::bank_account::{
    AccountOpenedEvent, BankAccountEvent, FundsDepositedEvent, FundsWithdrawnEvent,
};

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
    type View = BankAccountView;

    async fn handle(
        &mut self,
        id: String,
        event: BankAccountEvent,
        event_id: i64,
        _event_sequence: i64,
    ) -> Result<Self::View, Error> {
        use BankAccountEvent::*;

        let view = match event {
            AccountOpened(AccountOpenedEvent { initial_balance }) => BankAccountView {
                account_number: id,
                balance: initial_balance,
            },
            FundsDeposited(FundsDepositedEvent { amount }) => {
                let mut view = self.repository.load(&id, event_id).await?;
                view.balance += amount;
                view
            }
            FundsWithdrawn(FundsWithdrawnEvent { amount }) => {
                let mut view = self.repository.load(&id, event_id).await?;
                view.balance -= amount;
                view
            }
        };

        Ok(view)
    }

    async fn commit(
        &mut self,
        id: &str,
        view: Self::View,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error> {
        self.repository
            .save(id, Some(&view), event_id, event_sequence)
            .await
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
