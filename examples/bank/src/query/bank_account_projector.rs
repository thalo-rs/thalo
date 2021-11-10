use async_trait::async_trait;
use awto_es::{
    postgres::PgEventStore, AggregateType, Error, ErrorKind, EventHandler, EventStore,
    InternalError, Repository,
};
use bb8_postgres::tokio_postgres::{
    tls::{MakeTlsConnect, TlsConnect},
    Socket,
};

use crate::{
    command::{BankAccount, BankAccountEvent},
    query::{BankAccountView, BankAccountViewRepository},
};

pub struct BankAccountProjector<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    event_store: PgEventStore<Tls>,
    repository: BankAccountViewRepository<Tls>,
}

impl<Tls> BankAccountProjector<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pub fn new(event_store: PgEventStore<Tls>, repository: BankAccountViewRepository<Tls>) -> Self {
        Self {
            event_store,
            repository,
        }
    }
}

#[async_trait]
impl<Tls> EventHandler for BankAccountProjector<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Id = String;
    type Event = BankAccountEvent;

    async fn handle(
        &mut self,
        id: String,
        event: BankAccountEvent,
        event_sequence: i64,
    ) -> Result<(), Error> {
        use BankAccountEvent::*;

        match event {
            AccountOpened { initial_balance } => {
                let view = BankAccountView::new(id, initial_balance);
                self.repository.save(&view, event_sequence).await?;
            }
            FundsDeposited { amount } => {
                let (mut view, last_event_sequence) =
                    self.repository.load(&id).await?.ok_or_else(|| {
                        Error::new(ErrorKind::ResourceNotFound, "bank account not found")
                    })?;
                if last_event_sequence < event_sequence - 1 {
                    let missing_events = self
                        .event_store
                        .get_events(
                            BankAccount::aggregate_type(),
                            &id,
                            last_event_sequence + 1..event_sequence + 1,
                        )
                        .await?;
                    println!("Missing some events... {:#?}", missing_events);

                    for missing_event in missing_events {
                        let event = serde_json::from_value(missing_event.event_data)
                            .internal_error("corrupt event")?;
                        self.handle(id.clone(), event, missing_event.sequence)
                            .await?;
                    }
                } else {
                    view.balance += amount;
                    self.repository.save(&view, event_sequence).await?;
                }
            }
            FundsWithdrawn { amount } => {
                let (mut view, last_event_sequence) =
                    self.repository.load(&id).await?.ok_or_else(|| {
                        Error::new(ErrorKind::ResourceNotFound, "bank account not found")
                    })?;
                if last_event_sequence < event_sequence - 1 {
                    let missing_events = self
                        .event_store
                        .get_events(
                            BankAccount::aggregate_type(),
                            &id,
                            last_event_sequence + 1..event_sequence + 1,
                        )
                        .await?;
                    println!("Missing some events... {:#?}", missing_events);

                    for missing_event in missing_events {
                        let event = serde_json::from_value(missing_event.event_data)
                            .internal_error("corrupt event")?;
                        self.handle(id.clone(), event, missing_event.sequence)
                            .await?;
                    }
                } else {
                    view.balance -= amount;
                    self.repository.save(&view, event_sequence).await?;
                }
            }
        }
        Ok(())
    }
}
