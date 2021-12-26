//! Macros for thalo.

#![deny(missing_docs)]

use helpers::declare_derive_macro;

mod derives;
mod helpers;
mod traits;

/// Implements [`Aggregate`](thalo::aggregate::Aggregate) for a given struct.
///
/// The implementation of [`Aggregate::new`](thalo::aggregate::Aggregate::new) requires the struct to implement [`Default`].
///
/// # Container Attributes
///
/// - `#[thalo(event = "EventEnum")`
///
///   Specify the event enum used for [`Aggregate::Event`](thalo::aggregate::Aggregate::Event).
///
///   Defaults to struct name + `Event`. Eg: `struct BankAccount` would default to `BankAccountEvent`.
///
/// - `#[thalo(apply = "path")]`
///
///   Specify the path to the apply function.
///
///   Apply functions should have the signature:
///     ```
///     fn apply(aggregate: &mut Aggregate, event: AggregateEvent)
///     ```
///
///   Where `Aggregate` is the struct for your aggregate, and `AggregateEvent` is the [`Aggregate::Event`](thalo::aggregate::Aggregate::Event).
///
///   Defaults to `"handle"`.
///
/// # Field Attributes
///
/// - `#[thalo(id)]`
///
///   Specify which field is the [`Aggregate::ID`](thalo::aggregate::Aggregate::ID).
///
///   If this attribute is not present, it defaults to the first field that is named `id`,
///   otherwise a compile error will occur.
///
/// # Examples
///
/// Example without any attributes. All attributes are detected due to:
///
/// - Event enum is named `BankAccountEvent`
/// - Aggregate has a field named `id`
/// - There is an `apply` function in the current scope
///
/// ```
/// #[derive(Default, Aggregate, TypeId)]
/// struct BankAccount {
///     id: String,
///     balance: f64,
/// }
///
/// fn apply(bank_account: &mut BankAccount, event: BankAccountEvent) {
///     use BankAccountEvent::*;
///
///     match event {
///         OpenedAccount { balance } => {
///             bank_account.balance = balance;
///         }
///         DepositedFunds { amount } => {
///             bank_account.balance += amount;
///         }
///         WithdrewFunds { amount } => {
///             bank_account.balance -= amount;
///         }
///     }
/// }
/// ```
///
///  Example with attributes.
///
/// ```
/// #[derive(Default, Aggregate, TypeId)]
/// #[thalo(event = "BankEvent", apply = "apply_event")]
/// struct BankAccount {
///     #[thalo(id)]
///     account_id: String,
///     balance: f64,
/// }
///
/// fn apply_event(bank_account: &mut BankAccount, event: BankEvent) {
///     use BankEvent::*;
///
///     match event {
///         OpenedAccount { balance } => {
///             bank_account.balance = balance;
///         }
///         DepositedFunds { amount } => {
///             bank_account.balance += amount;
///         }
///         WithdrewFunds { amount } => {
///             bank_account.balance -= amount;
///         }
///     }
/// }
/// ```
#[proc_macro_derive(Aggregate, attributes(thalo))]
#[allow(non_snake_case)]
pub fn aggregate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    declare_derive_macro::<derives::Aggregate>(input)
}

/// Implements [`TypeId`](thalo::aggregate::TypeId) based on the struct/enum name as [snake_case](heck::ToSnakeCase).
///
/// # Container Attributes
///
/// - `#[thalo(type_id = "name")]`
///
///   Rename the type id to a custom string.
///
/// # Examples
///
/// ```
/// use thalo::aggregate::TypeId;
///
/// #[derive(TypeId)]
/// #[thalo(type_id = "bank_account_aggregate")]
/// struct BankAccount;
/// ```
#[proc_macro_derive(TypeId, attributes(thalo))]
#[allow(non_snake_case)]
pub fn type_id(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    declare_derive_macro::<derives::TypeId>(input)
}
