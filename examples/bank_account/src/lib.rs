use serde::{Deserialize, Serialize};
use thalo::{export_aggregate, Aggregate, Context, Error};

export_aggregate!(BankAccount);

#[derive(Aggregate, Serialize, Deserialize)]
#[aggregate(schema = "examples/bank_account/bank_account.esdl")]
pub struct BankAccount {
    id: String,
    opened: bool,
    balance: i64,
}

impl BankAccountAggregate for BankAccount {
    fn new(id: String) -> Result<Self, thalo::Error> {
        Ok(BankAccount {
            id,
            opened: false,
            balance: 0,
        })
    }

    fn apply_opened_account(&mut self, _ctx: Context, event: OpenedAccount) {
        self.opened = true;
        self.balance = event.initial_balance;
    }

    fn apply_deposited_funds(&mut self, _ctx: Context, event: DepositedFunds) {
        self.balance += event.amount as i64;
    }

    fn apply_withdrew_funds(&mut self, _ctx: Context, event: WithdrewFunds) {
        self.balance -= event.amount as i64;
    }

    fn handle_open_account(
        &self,
        _ctx: &mut Context,
        initial_balance: i64,
    ) -> Result<OpenedAccount, Error> {
        if self.opened {
            return Err(Error::fatal("account already opened"));
        }

        Ok(OpenedAccount { initial_balance })
    }

    fn handle_deposit_funds(
        &self,
        _ctx: &mut Context,
        amount: i32,
    ) -> Result<DepositedFunds, Error> {
        if !self.opened {
            return Err(Error::fatal("account not open"));
        }

        if amount < 0 {
            return Err(Error::fatal("cannot deposit negative funds"));
        }

        Ok(DepositedFunds { amount })
    }

    fn handle_withdraw_funds(
        &self,
        _ctx: &mut Context,
        amount: i32,
    ) -> Result<WithdrewFunds, Error> {
        if !self.opened {
            return Err(Error::fatal("account not open"));
        }

        if amount < 0 {
            return Err(Error::fatal("cannot withdraw negative funds"));
        }

        let new_balance = self.balance - amount as i64;
        if new_balance < 0 {
            return Err(Error::fatal("insufficient balance"));
        }

        Ok(WithdrewFunds { amount })
    }
}
