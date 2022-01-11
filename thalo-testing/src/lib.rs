//! Testing utilities for [thalo](https://docs.rs/thalo) apps.
//!
//! # Examples
//!
//! Create aggregate and events.
//!
//! ```
//! use thalo::{
//!     aggregate::{Aggregate, TypeId},
//!     event::EventType,
//! };
//! use thiserror::Error;
//!
//! #[derive(Aggregate, Clone, Debug, Default, PartialEq, TypeId)]
//! struct BankAccount {
//!     id: String,
//!     opened: bool,
//!     balance: f64,
//! }
//!
//! #[derive(Clone, Debug, EventType)]
//! enum BankAccountEvent {
//!     OpenedAccount { balance: f64 },
//! }
//!
//! fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
//!     use BankAccountEvent::*;
//!
//!     match event {
//!         OpenedAccount { balance } => {
//!             bank_account.opened = true;
//!             bank_account.balance = balance;
//!         }
//!     }
//! }
//! ```
//!
//! Test aggregate events.
//!
//! ```
//! # use thalo::{
//! #     aggregate::{Aggregate, TypeId},
//! #     event::EventType,
//! # };
//! # use thiserror::Error;
//! #
//! # #[derive(Aggregate, Clone, Debug, Default, PartialEq, TypeId)]
//! # struct BankAccount {
//! #     id: String,
//! #     opened: bool,
//! #     balance: f64,
//! # }
//! #
//! # #[derive(Clone, Debug, EventType)]
//! # enum BankAccountEvent {
//! #     OpenedAccount { balance: f64 },
//! # }
//! #
//! # fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
//! #     use BankAccountEvent::*;
//! #
//! #     match event {
//! #         OpenedAccount { balance } => {
//! #             bank_account.opened = true;
//! #             bank_account.balance = balance;
//! #         }
//! #     }
//! # }
//! #
//! #[cfg(test)]
//! mod tests {
//!     use thalo_testing::*;
//!     use super::{BankAccount, BankAccountEvent};
//!
//!     #[test]
//!     fn opened_account() {
//!         BankAccount::given(
//!             "account-123",
//!             BankAccountEvent::OpenedAccount {
//!                 balance: 0.0,
//!             }
//!         )
//!         .should_eq(BankAccount {
//!             id: "account-123".to_string(),
//!             opened: true,
//!             balance: 0.0,
//!         });
//!     }
//! }
//! ```
//!
//! Test aggregate commands.
//!
//! ```
//! # use thalo::{
//! #     aggregate::{Aggregate, TypeId},
//! #     event::EventType,
//! # };
//! # use thiserror::Error;
//! #
//! # #[derive(Aggregate, Clone, Debug, Default, PartialEq, TypeId)]
//! # struct BankAccount {
//! #     id: String,
//! #     opened: bool,
//! #     balance: f64,
//! # }
//! #
//! # #[derive(Clone, Debug, EventType)]
//! # enum BankAccountEvent {
//! #     OpenedAccount { balance: f64 },
//! # }
//! #
//! # fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
//! #     use BankAccountEvent::*;
//! #
//! #     match event {
//! #         OpenedAccount { balance } => {
//! #             bank_account.opened = true;
//! #             bank_account.balance = balance;
//! #         }
//! #     }
//! # }
//! #
//! impl BankAccount {
//!     pub fn open_account(
//!         &self,
//!         initial_balance: f64,
//!     ) -> Result<BankAccountEvent, BankAccountError> {
//!         if self.opened {
//!             return Err(BankAccountError::AlreadyOpened);
//!         }
//!
//!         if initial_balance < 0.0 {
//!             return Err(BankAccountError::NegativeAmount);
//!         }
//!
//!         Ok(BankAccountEvent::OpenedAccount {
//!             balance: initial_balance,
//!         })
//!     }
//! }
//!
//! #[derive(Debug, Error)]
//! pub enum BankAccountError {
//!     #[error("account already opened")]
//!     AlreadyOpened,
//!     #[error("negative amount")]
//!     NegativeAmount,
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     use thalo_testing::*;
//!     use super::{BankAccount, BankAccountError, BankAccountEvent};
//!
//!     #[test]
//!     fn open_account() {
//!         BankAccount::given_no_events("account-123")
//!             .when(|bank_account| bank_account.open_account(0.0))
//!             .then(Ok(BankAccountEvent::OpenedAccount {
//!                 balance: 0.0,
//!             }));
//!     }

//!     #[test]
//!     fn open_account_already_opened() {
//!         BankAccount::given(
//!             "account-123",
//!             BankAccountEvent::OpenedAccount {
//!                 balance: 0.0,
//!             },
//!         )
//!         .when(|bank_account| bank_account.open_account(50.0))
//!         .then(Err(BankAccountError::AlreadyOpened));
//!     }
//!
//!     #[test]
//!     fn open_account_negative_amount() {
//!         BankAccount::given_no_events()
//!             .when(|bank_account| bank_account.open_account(-10.0))
//!             .then(Err(BankAccountError::NegativeAmount));
//!     }
//! ```

#![deny(missing_docs)]

use std::fmt;

use thalo::aggregate::Aggregate;

/// An aggregate given events.
pub struct GivenTest<A>(A);

/// An aggregate when a command is performed.
pub struct WhenTest<A, R> {
    aggregate: A,
    result: R,
}

/// Given events for an aggregate.
pub trait Given: Aggregate + Sized {
    /// Given a single event for an aggregate.
    fn given(
        id: impl Into<<Self as Aggregate>::ID>,
        event: impl Into<<Self as Aggregate>::Event>,
    ) -> GivenTest<Self> {
        Self::given_events(id, vec![event.into()])
    }

    /// Given events for an aggregate.
    fn given_events(
        id: impl Into<<Self as Aggregate>::ID>,
        events: impl Into<Vec<<Self as Aggregate>::Event>>,
    ) -> GivenTest<Self> {
        let mut aggregate = Self::new(id.into());
        for event in events.into() {
            aggregate.apply(event);
        }
        GivenTest(aggregate)
    }

    /// Given no events for an aggregate.
    fn given_no_events(id: impl Into<<Self as Aggregate>::ID>) -> GivenTest<Self> {
        let aggregate = Self::new(id.into());
        GivenTest(aggregate)
    }
}

impl<A> Given for A where A: Aggregate + Sized {}

impl<A> GivenTest<A>
where
    A: Aggregate,
{
    /// When a command is applied.
    pub fn when<F, R>(mut self, f: F) -> WhenTest<A, R>
    where
        F: FnOnce(&mut A) -> R,
    {
        let result = f(&mut self.0);
        WhenTest {
            aggregate: self.0,
            result,
        }
    }

    /// Given previous events, the aggregate should equal the given state.
    pub fn should_eq<S>(self, state: S) -> Self
    where
        A: fmt::Debug + PartialEq<S>,
        S: fmt::Debug,
    {
        assert_eq!(self.0, state);
        self
    }

    /// Given previous events, the aggregate's state should be unchanged.
    pub fn should_be_unchanged(self) -> Self
    where
        A: fmt::Debug + PartialEq<A>,
        <A as Aggregate>::ID: Clone,
    {
        assert_eq!(self.0, A::new(self.0.id().clone()));
        self
    }
}

impl<A, R> WhenTest<A, R>
where
    A: Aggregate,
{
    /// Get the inner result from the previous when() action.
    pub fn into_result(self) -> R {
        self.result
    }

    /// Get the inner aggregate.
    pub fn into_state(self) -> A {
        self.aggregate
    }

    /// Then the result of the previous when() action should equal the given parameter.
    pub fn then<T>(self, result: T) -> WhenTest<A, R>
    where
        R: fmt::Debug + PartialEq<T>,
        T: fmt::Debug,
    {
        assert_eq!(self.result, result);
        self
    }

    /// Apply result of previous when() action.
    pub fn apply<F, I>(mut self, f: F) -> GivenTest<A>
    where
        F: FnOnce(R) -> I,
        I: IntoIterator<Item = <A as Aggregate>::Event>,
    {
        let events = f(self.result).into_iter();
        for event in events {
            self.aggregate.apply(event);
        }
        GivenTest(self.aggregate)
    }
}

impl<A, R, E> WhenTest<A, Result<R, E>>
where
    A: Aggregate,
{
    /// Then the result of the previous when() action should be Ok(T), with T being equal the given parameter.
    pub fn then_ok<T>(self, result: T) -> WhenTest<A, R>
    where
        T: fmt::Debug,
        R: fmt::Debug,
        E: fmt::Debug,
        Result<R, E>: PartialEq<Result<T, E>>,
    {
        assert_eq!(self.result, Result::<T, E>::Ok(result));
        WhenTest {
            aggregate: self.aggregate,
            result: self.result.unwrap(),
        }
    }

    /// Then the result of the previous when() action should be Err(E), with E being equal the given parameter.
    pub fn then_err<T>(self, result: T) -> GivenTest<A>
    where
        T: fmt::Debug,
        R: fmt::Debug,
        E: fmt::Debug,
        Result<R, E>: PartialEq<Result<R, T>>,
    {
        assert_eq!(self.result, Result::<R, T>::Err(result));
        GivenTest(self.aggregate)
    }
}
