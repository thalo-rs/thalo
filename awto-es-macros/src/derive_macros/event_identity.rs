use heck::ShoutySnekCase;
use proc_macro2::TokenStream;
use quote::quote;

pub struct EventIdentity {
    ident: syn::Ident,
}

impl EventIdentity {
    fn expand_impl_event_identity(&self) -> syn::Result<TokenStream> {
        let Self { ident } = self;

        let event_type_string = ident.to_string().TO_SHOUTY_SNEK_CASE();
        let event_type_string = event_type_string.trim_end_matches("Event");

        Ok(quote!(
            impl ::awto_es::EventIdentity for #ident {
                fn event_type() -> &'static str {
                    #event_type_string
                }
            }
        ))
    }
}

impl EventIdentity {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(EventIdentity { ident })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_event_identity = self.expand_impl_event_identity()?;

        Ok(expanded_impl_event_identity)
    }
}
