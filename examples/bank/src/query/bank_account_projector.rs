use async_trait::async_trait;
use awto_es::{
    postgres::tls::NoTls, postgres::PgEventStore, Error, ErrorKind, EventHandler, Projection,
    Repository,
};

use crate::{
    command::BankAccountEvent,
    query::{BankAccountView, BankAccountViewRepository},
};

#[derive(Clone)]
pub struct BankAccountProjector {
    event_store: PgEventStore<NoTls>,
    repository: BankAccountViewRepository,
}

impl BankAccountProjector {
    pub fn new(event_store: PgEventStore<NoTls>, repository: BankAccountViewRepository) -> Self {
        Self {
            event_store,
            repository,
        }
    }

    async fn handle_view(
        &mut self,
        event: BankAccountEvent,
        event_id: i64,
        event_sequence: i64,
        mut view: BankAccountView,
    ) -> Result<(), Error> {
        use BankAccountEvent::*;

        match event {
            AccountOpened { .. } => unreachable!("should be handled by event handler"),
            FundsDeposited { amount } => {
                view.balance += amount;
            }
            FundsWithdrawn { amount } => {
                view.balance -= amount;
            }
        }

        self.repository.save(&view, event_id, event_sequence).await
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
                let view = BankAccountView::new(id, initial_balance);
                self.repository
                    .save(&view, event_id, event_sequence)
                    .await?;
            }
            _ => {
                let (view, _) = self.repository.load(&id).await?.ok_or_else(|| {
                    Error::new(ErrorKind::ResourceNotFound, "bank account not found")
                })?;

                self.handle_view(event, event_id, event_sequence, view)
                    .await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Projection for BankAccountProjector {
    async fn last_event_id(&self) -> Result<Option<i64>, Error> {
        self.repository.last_event_id().await
    }
}
