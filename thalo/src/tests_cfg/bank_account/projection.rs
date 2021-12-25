use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use prettytable::Table;

use crate::event::{EventEnvelope, EventHandler, Infallible};

use super::BankAccountEvent;

#[derive(Default)]
pub struct BankAccountProjection {
    view: Mutex<HashMap<String, BankAccountView>>,
}

impl BankAccountProjection {
    pub fn print_bank_accounts(&self) {
        let view = self.view.lock().unwrap();

        let mut table = Table::new();
        table.set_titles(
            [
                "ID",
                "Balance",
                "Total Deposited",
                "Total Withdrawn",
                "Number of Transactions",
            ]
            .into(),
        );

        if view.is_empty() {
            table.add_row(["", "", "", "", ""].into());
        } else {
            for (id, bank_account) in view.iter() {
                table.add_row(
                    ([
                        id.clone(),
                        format!("{:.2}", bank_account.balance),
                        format!("{:.2}", bank_account.total_deposited),
                        format!("{:.2}", bank_account.total_withdrawn),
                        format!("{}", bank_account.number_of_transactions),
                    ])
                    .into(),
                );
            }
        }

        table.printstd();
    }
}

pub struct BankAccountView {
    pub balance: f64,
    pub total_deposited: f64,
    pub total_withdrawn: f64,
    pub number_of_transactions: u64,
}

impl BankAccountProjection {
    fn handle_opened_account(&self, id: String, balance: f64) {
        let mut view = self.view.lock().unwrap();

        view.entry(id).or_insert(BankAccountView {
            balance,
            total_deposited: 0.0,
            total_withdrawn: 0.0,
            number_of_transactions: 0,
        });
    }

    fn handle_deposited_funds(&self, id: String, amount: f64) {
        let mut view = self.view.lock().unwrap();

        if let Some(mut bank_account) = view.get_mut(&id) {
            bank_account.balance += amount;
            bank_account.total_deposited += amount;
            bank_account.number_of_transactions += 1;
        }
    }

    fn handle_withdrew_funds(&self, id: String, amount: f64) {
        let mut view = self.view.lock().unwrap();

        if let Some(mut bank_account) = view.get_mut(&id) {
            bank_account.balance -= amount;
            bank_account.total_withdrawn += amount;
            bank_account.number_of_transactions += 1;
        }
    }
}

#[async_trait]
impl EventHandler<BankAccountEvent> for BankAccountProjection {
    type Error = Infallible;

    async fn handle(&self, event: EventEnvelope<BankAccountEvent>) -> Result<(), Self::Error> {
        use BankAccountEvent::*;

        match event.event {
            OpenedAccount { balance } => self.handle_opened_account(event.aggregate_id, balance),
            DepositedFunds { amount } => self.handle_deposited_funds(event.aggregate_id, amount),
            WithdrewFunds { amount } => self.handle_withdrew_funds(event.aggregate_id, amount),
        }

        Ok(())
    }
}
