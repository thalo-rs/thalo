use proc_macro2::TokenStream;
use quote::quote;

use crate::traits::DeriveMacro;

pub(crate) struct IntoIterator {
    ident: syn::Ident,
}

impl DeriveMacro for IntoIterator {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(IntoIterator { ident })
    }

    fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let expanded_impl_into_iterator = self.expand_impl_into_iterator();

        Ok(expanded_impl_into_iterator)
    }
}

impl IntoIterator {
    fn expand_impl_into_iterator(&self) -> TokenStream {
        let Self { ident } = self;

        quote! {
            #[automatically_derived]
            impl ::std::iter::IntoIterator for #ident {
                type Item = Self;

                type IntoIter = ::std::iter::Once<Self>;

                fn into_iter(self) -> Self::IntoIter {
                    ::std::iter::once(self)
                }
            }
        }
    }
}
