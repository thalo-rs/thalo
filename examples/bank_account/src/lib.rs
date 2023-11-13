use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate, Events};

export_aggregate!(BankAccount);

pub struct BankAccount {
    #[allow(unused)]
    id: String,
    opened: bool,
    balance: i64,
}

impl Aggregate for BankAccount {
    type ID = String;
    type Command = BankAccountCommand;
    type Events = BankAccountEvents;
    type Error = &'static str;

    fn init(id: Self::ID) -> Self {
        BankAccount {
            id,
            opened: false,
            balance: 0,
        }
    }

    fn apply(&mut self, event: <Self::Events as Events>::Event) {
        use Event::*;

        match event {
            OpenedAccount(OpenedAccountV1 { initial_balance }) => {
                self.opened = true;
                self.balance = initial_balance as i64;
            }
            DepositedFunds(DepositedFundsV1 { amount }) => {
                self.balance += amount as i64;
            }
            WithdrewFunds(WithdrewFundsV1 { amount }) => {
                self.balance -= amount as i64;
            }
        }
    }

    fn handle(
        &self,
        cmd: Self::Command,
    ) -> Result<Vec<<Self::Events as Events>::Event>, Self::Error> {
        use BankAccountCommand::*;
        use Event::*;

        match cmd {
            OpenAccount { initial_balance } => {
                if self.opened {
                    return Err("account already opened");
                }

                Ok(vec![OpenedAccount(OpenedAccountV1 { initial_balance })])
            }
            DepositFunds { amount } => {
                if !self.opened {
                    return Err("account not open");
                }

                if amount == 0 {
                    return Err("cannot deposit 0");
                }

                Ok(vec![DepositedFunds(DepositedFundsV1 { amount })])
            }
            WithdrawFunds { amount } => {
                if !self.opened {
                    return Err("account not open");
                }

                if amount == 0 {
                    return Err("cannot withdraw 0");
                }

                let new_balance = self.balance - amount as i64;
                if new_balance < 0 {
                    return Err("insufficient balance");
                }

                Ok(vec![WithdrewFunds(WithdrewFundsV1 { amount })])
            }
        }
    }
}

#[derive(Deserialize)]
pub enum BankAccountCommand {
    OpenAccount { initial_balance: u32 },
    DepositFunds { amount: u32 },
    WithdrawFunds { amount: u32 },
}

#[derive(Events)]
pub enum BankAccountEvents {
    OpenedAccount(OpenedAccountV1),
    DepositedFunds(DepositedFundsV1),
    WithdrewFunds(WithdrewFundsV1),
}

#[derive(Serialize, Deserialize)]
pub struct OpenedAccountV1 {
    initial_balance: u32,
}

#[derive(Serialize, Deserialize)]
pub struct DepositedFundsV1 {
    amount: u32,
}

#[derive(Serialize, Deserialize)]
pub struct WithdrewFundsV1 {
    amount: u32,
}
