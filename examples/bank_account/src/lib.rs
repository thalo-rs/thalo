use serde::{Deserialize, Serialize};
use thalo::{events, export_aggregate, Aggregate, Apply, Command, Event, Handle};
use thiserror::Error;

export_aggregate!(BankAccount);

pub struct BankAccount {
    opened: bool,
    balance: i64,
}

impl Aggregate for BankAccount {
    type Command = BankAccountCommand;
    type Event = BankAccountEvent;

    fn init(_id: String) -> Self {
        BankAccount {
            opened: false,
            balance: 0,
        }
    }
}

#[derive(Command, Deserialize)]
pub enum BankAccountCommand {
    OpenAccount(OpenAccount),
    DepositFunds(DepositFunds),
    WithdrawFunds(WithdrawFunds),
}

#[derive(Debug, Error)]
pub enum BankAccountError {
    #[error("account already opened")]
    AccountAlreadyOpened,
    #[error("account not open")]
    AccountNotOpen,
    #[error("cannot withdraw/deposit an amount of 0")]
    AmountIsZero,
    #[error("insufficient balance")]
    InsufficientBalance,
}

#[derive(Deserialize)]
pub struct OpenAccount {}

impl Handle<OpenAccount> for BankAccount {
    type Error = BankAccountError;

    fn handle(&self, _cmd: OpenAccount) -> Result<Vec<BankAccountEvent>, Self::Error> {
        if self.opened {
            return Err(BankAccountError::AccountAlreadyOpened);
        }

        events![AccountOpened {}]
    }
}

#[derive(Deserialize)]
pub struct DepositFunds {
    amount: u32,
}

impl Handle<DepositFunds> for BankAccount {
    type Error = BankAccountError;

    fn handle(&self, cmd: DepositFunds) -> Result<Vec<BankAccountEvent>, Self::Error> {
        if !self.opened {
            return Err(BankAccountError::AccountNotOpen);
        }

        if cmd.amount == 0 {
            return Err(BankAccountError::AmountIsZero);
        }

        events![FundsDeposited { amount: cmd.amount }]
    }
}

#[derive(Deserialize)]
pub struct WithdrawFunds {
    amount: u32,
}

impl Handle<WithdrawFunds> for BankAccount {
    type Error = BankAccountError;

    fn handle(&self, cmd: WithdrawFunds) -> Result<Vec<BankAccountEvent>, Self::Error> {
        if !self.opened {
            return Err(BankAccountError::AccountNotOpen);
        }

        if cmd.amount == 0 {
            return Err(BankAccountError::AmountIsZero);
        }

        let new_balance = self.balance - cmd.amount as i64;
        if new_balance < 0 {
            return Err(BankAccountError::InsufficientBalance);
        }

        events![FundsWithdrawn { amount: cmd.amount }]
    }
}

#[derive(Event, Serialize, Deserialize)]
pub enum BankAccountEvent {
    OpenedAccount(AccountOpened),
    DepositedFunds(FundsDeposited),
    WithdrewFunds(FundsWithdrawn),
}

#[derive(Serialize, Deserialize)]
pub struct AccountOpened {}

impl Apply<AccountOpened> for BankAccount {
    fn apply(&mut self, _event: AccountOpened) {
        self.opened = true;
    }
}

#[derive(Serialize, Deserialize)]
pub struct FundsDeposited {
    pub amount: u32,
}

impl Apply<FundsDeposited> for BankAccount {
    fn apply(&mut self, event: FundsDeposited) {
        self.balance += event.amount as i64;
    }
}

#[derive(Serialize, Deserialize)]
pub struct FundsWithdrawn {
    pub amount: u32,
}

impl Apply<FundsWithdrawn> for BankAccount {
    fn apply(&mut self, event: FundsWithdrawn) {
        self.balance -= event.amount as i64;
    }
}
