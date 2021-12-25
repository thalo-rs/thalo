use super::{BankAccount, BankAccountEvent};

pub trait BankAccountCommand {
    type Error;

    fn open_account(
        id: String,
        initial_balance: f64,
    ) -> Result<(BankAccount, BankAccountEvent), Self::Error>;

    fn deposit_funds(&self, amount: f64) -> Result<BankAccountEvent, Self::Error>;

    fn withdraw_funds(&self, amount: f64) -> Result<BankAccountEvent, Self::Error>;
}
