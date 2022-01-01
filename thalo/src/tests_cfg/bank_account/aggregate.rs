use crate as thalo;
use crate::aggregate::{Aggregate, TypeId};
use crate::tests_cfg::bank_account::{DepositedFundsEvent, OpenedAccountEvent, WithdrewFundsEvent};

use super::BankAccountEvent;

#[derive(Default, Aggregate, TypeId)]
pub struct BankAccount {
    pub id: String,
    pub balance: f64,
}

fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
    use BankAccountEvent::*;

    match event {
        OpenedAccount(OpenedAccountEvent { balance }) => {
            bank_account.balance = balance;
        }
        DepositedFunds(DepositedFundsEvent { amount }) => {
            bank_account.balance += amount;
        }
        WithdrewFunds(WithdrewFundsEvent { amount }) => {
            bank_account.balance -= amount;
        }
    }
}
