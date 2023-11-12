use events::DeriveEvents;

mod events;

#[proc_macro_derive(Events)]
pub fn events(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as DeriveEvents)
        .expand()
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
