mod derive_macros;
mod proc_macros;

#[proc_macro_derive(Identity, attributes(identity))]
pub fn identity(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derive_macros::Identity::new(input) {
        Ok(derive) => derive_macros::Identity::expand(derive)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}

#[proc_macro_derive(Command)]
pub fn command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derive_macros::Command::new(input) {
        Ok(derive) => derive_macros::Command::expand(derive)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}

#[proc_macro_derive(Event, attributes(aggregate))]
pub fn event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derive_macros::Event::new(input) {
        Ok(derive) => derive_macros::Event::expand(derive)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}

#[proc_macro_derive(AggregateType)]
pub fn aggregate_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derive_macros::AggregateType::new(input) {
        Ok(derive) => derive_macros::AggregateType::expand(derive)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}

#[proc_macro_attribute]
pub fn aggregate_events(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::ItemImpl);

    match proc_macros::AggregateEvents::new(input) {
        Ok(proc) => proc_macros::AggregateEvents::expand(proc)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}

#[proc_macro_attribute]
pub fn aggregate_commands(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::ItemImpl);

    match proc_macros::AggregateCommands::new(input) {
        Ok(proc) => proc_macros::AggregateCommands::expand(proc)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}
