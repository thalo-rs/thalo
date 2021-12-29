use thiserror::Error;
use tonic::{Code, Status};

use super::{
    BankAccount, BankAccountEvent, DepositedFundsEvent, OpenedAccountEvent, WithdrewFundsEvent,
};

impl BankAccount {
    // Doesn't have &self parameter, so it is the entry command.
    pub fn open_account(
        id: String,
        initial_balance: f64,
    ) -> Result<(BankAccount, BankAccountEvent), BankAccountError> {
        if initial_balance < 0.0 {
            return Err(BankAccountError::NegativeAmount);
        }

        let bank_account = BankAccount {
            id,
            balance: initial_balance,
        };

        Ok((
            bank_account,
            BankAccountEvent::OpenedAccount(OpenedAccountEvent {
                balance: initial_balance,
            }),
        ))
    }

    pub fn deposit_funds(&self, amount: f64) -> Result<BankAccountEvent, BankAccountError> {
        if amount == 0.0 {
            return Err(BankAccountError::ZeroAmount);
        } else if amount < 0.0 {
            return Err(BankAccountError::NegativeAmount);
        }

        Ok(BankAccountEvent::DepositedFunds(DepositedFundsEvent {
            amount,
        }))
    }

    pub fn withdraw_funds(&self, amount: f64) -> Result<BankAccountEvent, BankAccountError> {
        if amount == 0.0 {
            return Err(BankAccountError::ZeroAmount);
        } else if amount < 0.0 {
            return Err(BankAccountError::NegativeAmount);
        }

        let new_balance = self.balance - amount;
        if new_balance < 0.0 {
            return Err(BankAccountError::InsufficientFunds);
        }

        Ok(BankAccountEvent::WithdrewFunds(WithdrewFundsEvent {
            amount,
        }))
    }
}

#[derive(Debug, Error)]
pub enum BankAccountError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("negative amount")]
    NegativeAmount,
    #[error("zero amount")]
    ZeroAmount,
}

impl From<BankAccountError> for Status {
    fn from(err: BankAccountError) -> Self {
        use BankAccountError::*;

        let code = match err {
            InsufficientFunds => Code::FailedPrecondition,
            NegativeAmount => Code::InvalidArgument,
            ZeroAmount => Code::InvalidArgument,
        };

        Status::new(code, err.to_string())
    }
}
