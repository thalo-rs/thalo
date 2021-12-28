use proc_macro2::TokenStream;
use quote::quote;

use crate::traits::DeriveMacro;

pub(crate) struct IntoEvents {
    ident: syn::Ident,
}

impl DeriveMacro for IntoEvents {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(IntoEvents { ident })
    }

    fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let expanded_impl_into_events = self.expand_impl_into_events();

        Ok(expanded_impl_into_events)
    }
}

impl IntoEvents {
    fn expand_impl_into_events(&self) -> TokenStream {
        let Self { ident } = self;

        quote! {
            impl thalo::event::IntoEvents for #ident {
                type Event = Self;

                fn into_events(self) -> std::vec::Vec<Self::Event> {
                    vec![self]
                }
            }
        }
    }
}
