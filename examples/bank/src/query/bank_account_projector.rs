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
            AccountOpened { .. } => {}
            FundsDeposited { amount } => {
                view.balance += amount;
                self.repository
                    .save(&view, event_id, event_sequence)
                    .await?;
            }
            FundsWithdrawn { amount } => {
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
                // if last_event_sequence < event_sequence - 1 {
                //     let missing_events = self
                //         .event_store
                //         .get_aggregate_events(
                //             BankAccount::aggregate_type(),
                //             &id,
                //             last_event_sequence + 1..event_sequence + 1,
                //         )
                //         .await?;

                //     for missing_event in missing_events {
                //         let event = serde_json::from_value(missing_event.event_data)
                //             .internal_error("corrupt event")?;
                //         self.handle(id.clone(), event, missing_event.sequence)
                //             .await?;
                //     }
                // } else if last_event_sequence < event_sequence {
                self.handle_view(event, event_id, event_sequence, view)
                    .await?;
                // }
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
