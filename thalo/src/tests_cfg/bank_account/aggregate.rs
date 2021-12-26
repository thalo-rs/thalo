use crate as thalo;
use crate::aggregate::{Aggregate, TypeId};

use super::BankAccountEvent;

#[derive(Default, Aggregate, TypeId)]
pub struct BankAccount {
    pub id: String,
    pub balance: f64,
}

fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
    use BankAccountEvent::*;

    match event {
        OpenedAccount { balance } => {
            bank_account.balance = balance;
        }
        DepositedFunds { amount } => {
            bank_account.balance += amount;
        }
        WithdrewFunds { amount } => {
            bank_account.balance -= amount;
        }
    }
}
