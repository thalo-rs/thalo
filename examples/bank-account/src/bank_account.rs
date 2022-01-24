use thalo::{
    aggregate::{Aggregate, TypeId},
    include_aggregate,
};

include_aggregate!("BankAccount");

#[derive(Aggregate, Clone, Debug, Default, PartialEq, TypeId)]
pub struct BankAccount {
    id: String,
    opened: bool,
    balance: f64,
}

impl BankAccountCommand for BankAccount {
    type Error = Error;

    fn open_account(&self, initial_balance: f64) -> Result<OpenedAccountEvent, Error> {
        if self.opened {
            return Err(Error::AccountAlreadyOpened);
        }

        Ok(OpenedAccountEvent { initial_balance })
    }

    fn deposit_funds(&self, amount: f64) -> Result<DepositedFundsEvent, Error> {
        if !self.opened {
            return Err(Error::AccountNotOpen);
        }

        if amount <= 0.0 {
            return Err(Error::NegativeOrZeroAmount);
        }

        Ok(DepositedFundsEvent { amount })
    }

    fn withdraw_funds(&self, amount: f64) -> Result<WithdrewFundsEvent, Error> {
        if !self.opened {
            return Err(Error::AccountNotOpen);
        }

        if amount <= 0.0 {
            return Err(Error::NegativeOrZeroAmount);
        }

        let new_balance = self.balance - amount;
        if new_balance < 0.0 {
            return Err(Error::InsufficientBalance);
        }

        Ok(WithdrewFundsEvent { amount })
    }
}

fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
    use BankAccountEvent::*;

    match event {
        OpenedAccount(OpenedAccountEvent { initial_balance }) => {
            bank_account.opened = true;
            bank_account.balance = initial_balance;
        }
        DepositedFunds(DepositedFundsEvent { amount }) => {
            bank_account.balance += amount;
        }
        WithdrewFunds(WithdrewFundsEvent { amount }) => {
            bank_account.balance -= amount;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    AccountAlreadyOpened,
    AccountNotOpen,
    InsufficientBalance,
    NegativeOrZeroAmount,
}
