use thiserror::Error;
use tonic::{metadata::MetadataMap, Code, Status};

use crate::aggregate::{Aggregate, TypeId};

use super::{BankAccountCommand, BankAccountEvent};

pub struct BankAccount {
    id: String,
    balance: f64,
}

impl BankAccountCommand for BankAccount {
    type Error = BankAccountError;

    // Doesn't have &self parameter, so it is the entry command.
    fn open_account(id: String, initial_balance: f64) -> (BankAccount, BankAccountEvent) {
        let bank_account = BankAccount {
            id,
            balance: initial_balance,
        };

        (
            bank_account,
            BankAccountEvent::OpenedAccount {
                balance: initial_balance,
            },
        )
    }

    fn deposit_funds(&self, amount: f64) -> Result<BankAccountEvent, BankAccountError> {
        if amount == 0.0 {
            return Err(BankAccountError::ZeroAmount);
        } else if amount < 0.0 {
            return Err(BankAccountError::NegativeAmount);
        }

        Ok(BankAccountEvent::DepositedFunds { amount })
    }

    fn withdraw_funds(&self, amount: f64) -> Result<BankAccountEvent, BankAccountError> {
        if amount == 0.0 {
            return Err(BankAccountError::ZeroAmount);
        } else if amount < 0.0 {
            return Err(BankAccountError::NegativeAmount);
        }

        let new_balance = self.balance - amount;
        if new_balance < 0.0 {
            return Err(BankAccountError::InsufficientFunds);
        }

        Ok(BankAccountEvent::WithdrewFunds { amount })
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

impl Aggregate for BankAccount {
    type ID = String;
    type Event = BankAccountEvent;

    fn new(id: Self::ID) -> Self {
        BankAccount { id, balance: 0.0 }
    }

    fn id(&self) -> &Self::ID {
        &self.id
    }

    fn apply(&mut self, event: BankAccountEvent) {
        use BankAccountEvent::*;

        match event {
            OpenedAccount { balance } => {
                self.balance = balance;
            }
            DepositedFunds { amount } => {
                self.balance += amount;
            }
            WithdrewFunds { amount } => {
                self.balance -= amount;
            }
        }
    }
}

impl TypeId for BankAccount {
    fn type_id() -> &'static str {
        "bank_account"
    }
}

impl BankAccountError {
    pub fn code(&self) -> &'static str {
        use BankAccountError::*;

        match self {
            InsufficientFunds => "INSUFFICIENT_FUNDS",
            NegativeAmount => "NEGATIVE_AMOUNT",
            ZeroAmount => "ZERO_AMOUNT",
        }
    }
}

impl From<BankAccountError> for Status {
    fn from(err: BankAccountError) -> Self {
        use BankAccountError::*;

        let mut metadata = MetadataMap::new();
        metadata.insert("code", err.code().parse().unwrap());

        let code = match err {
            InsufficientFunds => Code::FailedPrecondition,
            NegativeAmount => Code::InvalidArgument,
            ZeroAmount => Code::InvalidArgument,
        };

        Status::with_metadata(code, err.to_string(), metadata)
    }
}
