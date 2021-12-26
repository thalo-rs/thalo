//! Macros for thalo.

#![deny(missing_docs)]

use helpers::declare_derive_macro;

mod derives;
mod helpers;
mod traits;

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
