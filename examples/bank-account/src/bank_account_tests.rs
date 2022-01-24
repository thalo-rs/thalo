use thalo_testing::*;

use crate::bank_account::{
    BankAccount, BankAccountCommand, DepositedFundsEvent, Error, OpenedAccountEvent,
    WithdrewFundsEvent,
};

const ACCOUNT_NAME: &str = "john-doe";

#[test]
fn open_account() {
    BankAccount::given_no_events(ACCOUNT_NAME.to_string())
        // Open account
        .when(|bank_account| bank_account.open_account(0.0))
        // Then ok
        .then_ok(OpenedAccountEvent {
            initial_balance: 0.0,
        })
        .apply()
        // Open account again
        .when(|bank_account| bank_account.open_account(0.0))
        // Then error
        .then_err(Error::AccountAlreadyOpened);
}

#[test]
fn deposit_funds() {
    BankAccount::given_no_events(ACCOUNT_NAME.to_string())
        // Deposit funds before opening account
        .when(|bank_account| bank_account.deposit_funds(10.0))
        // Then error
        .then_err(Error::AccountNotOpen)
        // Open account
        .when(|bank_account| bank_account.open_account(0.0))
        .then_ok(OpenedAccountEvent {
            initial_balance: 0.0,
        })
        .apply()
        // Deposit funds after opening account
        .when(|bank_account| bank_account.deposit_funds(10.0))
        // Then ok
        .then_ok(DepositedFundsEvent { amount: 10.0 })
        .apply()
        // Deposit 0.0
        .when(|bank_account| bank_account.deposit_funds(0.0))
        // Then error
        .then_err(Error::NegativeOrZeroAmount)
        // Deposit -50.0
        .when(|bank_account| bank_account.deposit_funds(-50.0))
        // Then error
        .then_err(Error::NegativeOrZeroAmount);
}

#[test]
fn withdraw_funds() {
    BankAccount::given_no_events(ACCOUNT_NAME.to_string())
        // Withdraw funds before opening account
        .when(|bank_account| bank_account.withdraw_funds(10.0))
        // Then error
        .then_err(Error::AccountNotOpen)
        // Open account
        .when(|bank_account| bank_account.open_account(0.0))
        .then_ok(OpenedAccountEvent {
            initial_balance: 0.0,
        })
        .apply()
        // Withdraw funds before depositing
        .when(|bank_account| bank_account.withdraw_funds(10.0))
        // Then error
        .then_err(Error::InsufficientBalance)
        // Deposit 50.0
        .when(|bank_account| bank_account.deposit_funds(50.0))
        // Then ok
        .then_ok(DepositedFundsEvent { amount: 50.0 })
        .apply()
        // Withdraw 45.0
        .when(|bank_account| bank_account.withdraw_funds(45.0))
        // Then ok
        .then_ok(WithdrewFundsEvent { amount: 45.0 })
        .apply()
        // Withdraw 6.0
        .when(|bank_account| bank_account.withdraw_funds(6.0))
        // Then error
        .then_err(Error::InsufficientBalance)
        // Withdraw 0.0
        .when(|bank_account| bank_account.withdraw_funds(0.0))
        // Then error
        .then_err(Error::NegativeOrZeroAmount)
        // Withdraw -20.0
        .when(|bank_account| bank_account.withdraw_funds(-20.0))
        // Then error
        .then_err(Error::NegativeOrZeroAmount);
}
