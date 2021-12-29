use better_bae::{FromAttributes, TryFromAttributes};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::traits::DeriveMacro;

pub(crate) struct IntoIterator {
    ident: syn::Ident,
    parent_variant: Option<(syn::Ident, Option<syn::Ident>)>,
}

#[derive(Default, FromAttributes)]
#[bae("thalo")]
struct Attrs {
    parent: Option<syn::LitStr>,
    variant: Option<syn::LitStr>,
}

impl DeriveMacro for IntoIterator {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::try_from_attributes(&input.attrs)?.unwrap_or_default();
        let ident = input.ident;

        let parent_variant = if let Some(parent) = attrs.parent {
            Some((
                format_ident!("{}", parent.value()),
                attrs
                    .variant
                    .map(|variant| format_ident!("{}", variant.value())),
            ))
        } else if let Some(variant) = attrs.variant {
            return Err(syn::Error::new(
                variant.span(),
                "variant specified, but missing parent",
            ));
        } else {
            None
        };

        Ok(IntoIterator {
            ident,
            parent_variant,
        })
    }

    fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let expanded_impl_into_iterator = self.expand_impl_into_iterator();

        Ok(expanded_impl_into_iterator)
    }
}

impl IntoIterator {
    fn expand_impl_into_iterator(&self) -> TokenStream {
        let Self {
            ident,
            parent_variant,
        } = self;

        if let Some((parent, variant)) = parent_variant {
            let variant = variant.clone().unwrap_or_else(|| ident.clone());

            quote! {
                #[automatically_derived]
                impl ::std::iter::IntoIterator for #ident {
                    type Item = #parent;

                    type IntoIter = ::std::iter::Once<Self::Item>;

                    fn into_iter(self) -> Self::IntoIter {
                        ::std::iter::once(#parent::#variant(self))
                    }
                }
            }
        } else {
            quote! {
                #[automatically_derived]
                impl ::std::iter::IntoIterator for #ident {
                    type Item = Self;

                    type IntoIter = ::std::iter::Once<Self::Item>;

                    fn into_iter(self) -> Self::IntoIter {
                        ::std::iter::once(self)
                    }
                }
            }
        }
    }
}
