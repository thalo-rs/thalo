//! Define aggregates as schema files and compile into Rust code.
//!
//! # Example
//!
//! Yaml schema
//!
//! ```yaml
//! # The name of the aggregate
//! name: BankAccount
//!
//! # The events resulted resulting from commands
//! events:
//!   AccountOpened:
//!     fields:
//!       - name: initial_balance
//!         type: f64
//!
//!   DepositedFunds:
//!     fields:
//!       - name: amount
//!         type: f64
//!
//!   WithdrewFunds:
//!     fields:
//!       - name: amount
//!         type: f64
//!
//! # The commands availble for the aggregate
//! commands:
//!   open_account:
//!     event: AccountOpened # resulting event when command is successful
//!     args:
//!       - name: initial_balance
//!         type: f64
//!
//!   deposit_funds:
//!     event: DepositedFunds
//!     args:
//!       - name: amount
//!         type: f64
//!
//!   withdraw_funds:
//!     event: WithdrewFunds
//!     args:
//!       - name: amount
//!         type: f64
//!
//! # Errors that can occur when executing a command
//! errors:
//!   NegativeAmount:
//!     message: "amount cannot be negative"
//!
//!   InsufficientFunds:
//!     message: "insufficient funds for transaction"
//! ```
//!
//! Rust implementation
//!
//! ```
//! use thalo::aggregate::{Aggregate, TypeId};
//!
//! // Creates BankAccountCommand, BankAccountEvent (and individual events), BankAccountError
//! include_aggregate!("BankAccount");
//!
//! #[derive(Aggregate, Clone, Debug, Default, PartialEq, TypeId)]
//! pub struct BankAccount {
//!     id: String,
//!     balance: bool,
//! }
//!
//! impl BankAccountCommand for BankAccount {
//!     fn open_account(&self, initial_balance: f64) -> Result<OpenedAccountEvent, BankAccountError> {
//!         todo!()
//!     }
//!
//!     fn deposit_funds(&self, amount: f64) -> Result<DepositedFundsEvent, BankAccountError> {
//!         todo!()
//!     }
//!
//!     fn withdraw_funds(&self, amount: f64) -> Result<WithdrewFundsEvent, BankAccountError> {
//!         todo!()
//!     }
//! }
//!
//! fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
//!     use BankAccountEvent::*;
//!
//!     match event {
//!         OpenedAccount(_) => { todo!() },
//!         DepositedFunds(_) => { todo!() },
//!         WithdrewFunds(_) => { todo!() },
//!     }
//! }
//! ```

#![deny(missing_docs)]

pub use compiler::Compiler;
pub use error::Error;

mod compiler;
mod error;
pub mod schema;

/// Configure a compiler.
pub fn configure() -> Compiler {
    Compiler::new()
}
