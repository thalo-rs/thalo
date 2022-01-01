use better_bae::{FromAttributes, TryFromAttributes};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::traits::DeriveMacro;

pub(crate) struct Event {
    attrs: Attrs,
    ident: syn::Ident,
}

#[derive(FromAttributes)]
#[bae("thalo")]
struct Attrs {
    parent: syn::LitStr,
    variant: Option<syn::LitStr>,
}

impl DeriveMacro for Event {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::from_attributes(&input.attrs)?;
        let ident = input.ident;

        Ok(Event { attrs, ident })
    }

    fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let expanded_impl_into_event = self.expand_impl_into_event();

        Ok(expanded_impl_into_event)
    }
}

impl Event {
    fn expand_impl_into_event(&self) -> TokenStream {
        let Self { attrs, ident } = self;

        let parent = format_ident!("{}", attrs.parent.value());

        let variant = attrs
            .variant
            .as_ref()
            .map(|variant| format_ident!("{}", variant.value()))
            .unwrap_or_else(|| ident.clone());

        quote! {
            #[automatically_derived]
            impl ::std::convert::From<#ident> for #parent {
                fn from(ev: #ident) -> Self {
                    #parent::#variant(ev)
                }
            }

            #[automatically_derived]
            impl thalo::event::IntoEvents<#parent> for #ident {
                fn into_events(self) -> ::std::vec::Vec<#parent> {
                    vec![#parent::#variant(self)]
                }
            }
        }
    }
}
