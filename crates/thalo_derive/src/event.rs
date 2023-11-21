use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    ItemEnum,
};

pub struct DeriveEvent {
    ident: syn::Ident,
    events: HashMap<syn::Ident, syn::Path>,
}

impl Parse for DeriveEvent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_enum: ItemEnum = input.parse()?;
        let events = item_enum
            .variants
            .into_iter()
            .map(|variant| {
                let name = variant.ident;
                let path = match variant.fields {
                    syn::Fields::Named(_) => {
                        return Err(syn::Error::new(
                            variant.fields.span(),
                            format!("event must be an unnamed field"),
                        ));
                    }
                    syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) => {
                        let span = unnamed.span();
                        let mut iter = unnamed.into_iter();
                        let Some(field) = iter.next() else {
                            return Err(syn::Error::new(span, "event not specified"));
                        };
                        let syn::Type::Path(syn::TypePath { path, .. }) = field.ty else {
                            return Err(syn::Error::new(span, "expected path to event"));
                        };
                        if iter.next().is_some() {
                            return Err(syn::Error::new(span, "only one event can be specified"));
                        }
                        path
                    }
                    syn::Fields::Unit => {
                        return Err(syn::Error::new(
                            variant.fields.span(),
                            format!("inner event type must be specified"),
                        ));
                    }
                };
                Ok((name, path))
            })
            .collect::<Result<_, _>>()?;

        Ok(DeriveEvent {
            ident: item_enum.ident,
            events,
        })
    }
}

impl DeriveEvent {
    pub fn expand(self) -> TokenStream {
        let apply_impl = self.expand_apply_impl();
        let from_impls = self.expand_from_impls();

        quote! {
            #apply_impl
            #from_impls
        }
    }

    fn expand_apply_impl(&self) -> TokenStream {
        let Self { ident, events, .. } = self;

        let paths = events.values();
        let arms = events.iter().map(|(name, path)| {
            quote! {
                #ident::#name(event) => <T as ::thalo::Apply<#path>>::apply(&mut self.0, event)
            }
        });

        quote! {
            #[automatically_derived]
            impl<T> ::thalo::Apply<#ident> for ::thalo::State<T>
            where
                T: ::thalo::Aggregate,
                #( T: ::thalo::Apply<#paths>, )*
            {
                fn apply(&mut self, event: #ident) {
                    match event {
                        #( #arms, )*
                    }
                }
            }
        }
    }

    fn expand_from_impls(&self) -> TokenStream {
        let Self { ident, events, .. } = self;

        let from_impls = events.iter().map(|(name, path)| {
            quote! {
                #[automatically_derived]
                impl ::std::convert::From<#path> for #ident {
                    fn from(event: #path) -> Self {
                        #ident::#name(event)
                    }
                }
            }
        });

        quote! {
            #( #from_impls )*
        }
    }
}
