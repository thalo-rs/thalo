use thalo::{commands, events, Aggregate, Error};

#[derive(Aggregate, Clone, Debug, Default)]
pub struct BankAccount {
    #[identity]
    account_number: String,
    balance: f64,
    opened: bool,
}

#[commands]
impl BankAccount {
    /// Creates a command for opening an account
    pub fn open_account(&self, initial_balance: f64) -> Result<AccountOpenedEvent, Error> {
        if self.opened {
            return Err(Error::invariant_code("ACCOUNT_ALREADY_OPENED"));
        }

        // Reference the event created by the BankAccount::account_opened method
        Ok(AccountOpenedEvent { initial_balance })
    }

    /// Deposit funds
    pub fn deposit_funds(&self, amount: f64) -> Result<FundsDepositedEvent, Error> {
        if !self.opened {
            return Err(Error::invariant_code("ACCOUNT_DOES_NOT_EXIST"));
        }

        Ok(FundsDepositedEvent { amount })
    }

    /// Withdraw funds
    pub fn withdraw_funds(&self, amount: f64) -> Result<FundsWithdrawnEvent, Error> {
        if !self.opened {
            return Err(Error::invariant_code("ACCOUNT_DOES_NOT_EXIST"));
        }

        let new_balance = self.balance - amount;
        if new_balance < 0.0 {
            return Err(Error::invariant_code("INSUFFICIENT_FUNDS"));
        }

        Ok(FundsWithdrawnEvent { amount })
    }
}

#[events]
impl BankAccount {
    /// Creates an event for when a user opened an account
    pub fn account_opened(&mut self, initial_balance: f64) {
        self.balance = initial_balance;
        self.opened = true;
    }

    /// Account funds were deposited
    pub fn funds_deposited(&mut self, amount: f64) {
        self.balance += amount;
    }

    /// Account funds were withdrawn
    pub fn funds_withdrawn(&mut self, amount: f64) {
        self.balance -= amount;
    }
}
