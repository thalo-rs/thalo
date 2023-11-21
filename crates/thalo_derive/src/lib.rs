use command::DeriveCommand;
use event::DeriveEvent;

mod command;
mod event;

/// Used to implement traits for an aggregate command enum.
///
/// A command enum can either wrap nested command structs which each implement `Handle`,
/// or contain the data embedded without nested command structs.
///
/// Depeneding on this, the implementation for `thalo::Handle<...> for thalo::State<T>` will be slightly different.
/// However, the implementation should not concern developers, as this is only used internally by the
/// `thalo::export_aggregate!` macro.
///
/// If the command uses nested command structs, then a `From` implementation will be generated for each variant.
#[proc_macro_derive(Command)]
pub fn command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as DeriveCommand)
        .expand()
        .into()
}

/// Used to implement traits for an aggregate event enum.
///
/// Expands to the following:
///
/// - Implements `thalo::Apply<...> for thalo::State<T>`.
/// - Implements `From<#path> for #ident` for each variant.
#[proc_macro_derive(Event)]
pub fn event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as DeriveEvent)
        .expand()
        .into()
}
