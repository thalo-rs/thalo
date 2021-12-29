//! Macros for thalo.

#![deny(missing_docs)]

use helpers::declare_derive_macro;

mod derives;
mod helpers;
mod traits;

/// Implements [`Aggregate`](https://docs.rs/thalo/latest/thalo/aggregate/trait.Aggregate.html) for a given struct.
///
/// The implementation of [`Aggregate::new`](https://docs.rs/thalo/latest/thalo/aggregate/trait.Aggregate.html#tymethod.new) requires the struct to implement [`Default`].
///
/// # Container Attributes
///
/// - `#[thalo(event = "EventEnum")`
///
///   Specify the event enum used for [`Aggregate::Event`](https://docs.rs/thalo/latest/thalo/aggregate/trait.Aggregate.html#associatedtype.Event).
///
///   Defaults to struct name + `Event`. Eg: `struct BankAccount` would default to `BankAccountEvent`.
///
/// - `#[thalo(apply = "path")]`
///
///   Specify the path to the apply function.
///
///   Apply functions should have the signature:
///     ```ignore
///     fn apply(aggregate: &mut Aggregate, event: AggregateEvent)
///     ```
///
///   Where `Aggregate` is the struct for your aggregate, and `AggregateEvent` is the [`Aggregate::Event`](https://docs.rs/thalo/latest/thalo/aggregate/trait.Aggregate.html#associatedtype.Event).
///
///   Defaults to `"handle"`.
///
/// # Field Attributes
///
/// - `#[thalo(id)]`
///
///   Specify which field is the [`Aggregate::ID`](https://docs.rs/thalo/latest/thalo/aggregate/trait.Aggregate.html#associatedtype.ID).
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
/// Example with attributes.
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
pub fn aggregate(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    declare_derive_macro::<derives::Aggregate>(input)
}

/// Implements [`EventType`](https://docs.rs/thalo/latest/thalo/event/trait.EventType.html) for a given struct.
///
/// # Container Attributes
///
/// - `#[thalo(rename_all = "...")]`
///
///   Rename all variants according to the given case convention.
///
///   The possible values are:
///   
///   - `"lowercase"`
///   - `"UPPERCASE"`
///   - `"PascalCase"`
///   - `"camelCase"`
///   - `"snake_case"`
///   - `"SCREAMING_SNAKE_CASE"`
///   - `"kebab-case"`
///   - `"SCREAMING-KEBAB-CASE"`
///
/// # Variant Attributes
///
/// - `#[thalo(rename = "name")]`
///
///   Use the given name as the event type instead of it's Rust name.
///
/// # Examples
///
/// Example without any attributes. All variants are used as [`EventType::event_type`](https://docs.rs/thalo/latest/thalo/event/trait.EventType.html#tymethod.event_type) based on their Rust name.
///
/// ```
/// #[derive(EventType)]
/// pub enum BankAccountEvent {
///     OpenedAccount { balance: f64 },
///     DepositedFunds { amount: f64 },
///     WithdrewFunds { amount: f64 },
/// }
///
/// assert_eq!(BankAccountEvent::OpenedAccount { balance: 0.0 }.event_name(), "OpenedAccount");
/// ```
///
/// Example without any attributes. All variants are used as [`EventType::event_type`](https://docs.rs/thalo/latest/thalo/event/trait.EventType.html#tymethod.event_type) based on their Rust name.
///
/// ```
/// #[derive(EventType)]
/// #[thalo(rename_all = "SCREAMING_SNAKE_CASE")]
/// pub enum BankAccountEvent {
///     OpenedAccount { balance: f64 },
///     DepositedFunds { amount: f64 },
///     #[thalo(rename = "FUNDS_WITHDRAWN")]
///     WithdrewFunds { amount: f64 },
/// }
///
/// assert_eq!(BankAccountEvent::OpenedAccount { balance: 0.0 }.event_name(), "OPENED_ACCOUNT");
/// ```
#[proc_macro_derive(EventType, attributes(thalo))]
pub fn event_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    declare_derive_macro::<derives::EventType>(input)
}

/// Implements [`TypeId`](https://docs.rs/thalo/latest/thalo/aggregate/trait.TypeId.html) based on the struct/enum name as [snake_case](heck::ToSnakeCase).
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
pub fn type_id(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    declare_derive_macro::<derives::TypeId>(input)
}
