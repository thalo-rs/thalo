use awto_es::{
    macros::{aggregate_commands, aggregate_events, AggregateType, Identity},
    Error,
};

#[derive(Identity, AggregateType, Clone, Debug, Default)]
pub struct BankAccount {
    #[identity]
    account_number: String,
    balance: f64,
    opened: bool,
}

#[aggregate_events]
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

#[aggregate_commands]
impl BankAccount {
    /// Creates a command for opening an account
    pub fn open_account(&self, initial_balance: f64) -> Result<Vec<BankAccountEvent>, Error> {
        if self.opened {
            return Err(Error::validation_error("account already opened"));
        }

        // Reference the event created by the BankAccount::account_opened method
        Ok(vec![BankAccountEvent::AccountOpened { initial_balance }])
    }

    /// Deposit funds
    pub fn deposit_funds(&self, amount: f64) -> Result<Vec<BankAccountEvent>, Error> {
        if !self.opened {
            return Err(Error::validation_error("account does not exist"));
        }

        Ok(vec![BankAccountEvent::FundsDeposited { amount }])
    }

    /// Withdraw funds
    pub fn withdraw_funds(&self, amount: f64) -> Result<Vec<BankAccountEvent>, Error> {
        if !self.opened {
            return Err(Error::validation_error("account does not exist"));
        }

        let new_balance = self.balance - amount;
        if new_balance < 0.0 {
            return Err(Error::validation_error("insufficient funds for withdrawal"));
        }

        Ok(vec![BankAccountEvent::FundsWithdrawn { amount }])
    }
}
