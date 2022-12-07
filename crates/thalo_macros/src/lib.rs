mod aggregate;
mod commands;
mod event_collection;
mod events;

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use syn::{parse_macro_input, DeriveInput};

use crate::aggregate::DeriveAggregate;
use crate::commands::DeriveCommands;
use crate::event_collection::DeriveEventCollection;
use crate::events::DeriveEvents;

#[proc_macro_error]
#[proc_macro_derive(Aggregate, attributes(aggregate))]
pub fn aggregate(input: TokenStream) -> TokenStream {
    expand_derive_macro::<DeriveAggregate>(input)
}

#[proc_macro_error]
#[proc_macro_derive(Events, attributes(events))]
pub fn events(input: TokenStream) -> TokenStream {
    expand_derive_macro::<DeriveEvents>(input)
}

#[proc_macro_error]
#[proc_macro_derive(EventCollection)]
pub fn event_collection(input: TokenStream) -> TokenStream {
    expand_derive_macro::<DeriveEventCollection>(input)
}

#[proc_macro_error]
#[proc_macro_derive(Commands, attributes(commands))]
pub fn commands(input: TokenStream) -> TokenStream {
    expand_derive_macro::<DeriveCommands>(input)
}

trait DeriveMacro: Sized {
    fn expand(&self) -> proc_macro2::TokenStream;
    fn parse_input(input: DeriveInput) -> syn::Result<Self>;
}

fn expand_derive_macro<T: DeriveMacro>(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let derive: T = match T::parse_input(input) {
        Ok(derive) => derive,
        Err(err) => abort!(err),
    };

    let expanded = derive.expand();
    TokenStream::from(expanded)
}
