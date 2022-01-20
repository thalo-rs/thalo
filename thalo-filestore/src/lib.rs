//! A filestore implementation of [EventStore](thalo::event_store::EventStore).
//!
//! Built on top of [thalo-inmemory](https://docs.rs/thalo-inmemory), events are persisted to a file.

pub use error::Error;
pub use event_store::FlatFileEventStore;

mod error;
mod event_store;

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::NamedTempFile;
    use thalo::{
        event_store::EventStore,
        tests_cfg::bank_account::{
            BankAccount, BankAccountEvent, DepositedFundsEvent, OpenedAccountEvent,
        },
    };

    use crate::FlatFileEventStore;

    #[tokio::test]
    async fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        let file_store = NamedTempFile::new()?.into_temp_path().to_path_buf();

        let event_store = FlatFileEventStore::load(file_store.clone())?;

        event_store.event_store.print();

        event_store
            .save_events::<BankAccount>(
                &"jimmy".to_string(),
                &[BankAccountEvent::OpenedAccount(OpenedAccountEvent {
                    balance: 0.0,
                })],
            )
            .await?;

        event_store
            .save_events::<BankAccount>(
                &"jimmy".to_string(),
                &[BankAccountEvent::DepositedFunds(DepositedFundsEvent {
                    amount: 24.0,
                })],
            )
            .await?;

        let file_content = fs::read_to_string(file_store.clone())?;
        println!("{}", file_content);

        FlatFileEventStore::load(file_store)?.event_store.print();

        Ok(())
    }
}
