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
    // event_type_ident: syn::Ident,
    events: HashMap<syn::Ident, syn::Path>,
}

impl Parse for DeriveEvent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_enum: ItemEnum = input.parse()?;
        // let event_type_ident = format_ident!("{}Type", item_enum.ident);
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
            // event_type_ident,
            events,
        })
    }
}

impl DeriveEvent {
    pub fn expand(self) -> TokenStream {
        let apply_impl = self.expand_apply_impl();
        let from_impls = self.expand_from_impls();
        // let event_name_impls = self.expand_event_name_impls();
        // let event_type = self.expand_event_type();
        // let event_type_display_impl = self.expand_event_type_display_impl();
        // let from_event_for_event_type_impls = self.expand_from_event_for_event_type();

        quote! {
            #apply_impl
            #from_impls
            // #event_name_impls
            // #event_type
            // #event_type_display_impl
            // #from_event_for_event_type_impls
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

    // fn expand_event_name_impls(&self) -> TokenStream {
    //     let Self { events, .. } = self;

    //     let event_name_impls = events.iter().map(|(name, path)| {
    //         let event_name = name.to_string();

    //         quote! {
    //             impl ::thalo::EventName for #path {
    //                 fn event_name() -> &'static str {
    //                     #event_name
    //                 }
    //             }
    //         }
    //     });

    //     quote! {
    //         #( #event_name_impls )*
    //     }
    // }

    // fn expand_event_type(&self) -> TokenStream {
    //     let Self {
    //         event_type_ident,
    //         events,
    //         ..
    //     } = self;

    //     let variants = events.keys();

    //     quote! {
    //         #[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
    //         pub enum #event_type_ident {
    //             #( #variants, )*
    //         }

    //     }
    // }

    // fn expand_event_type_display_impl(&self) -> TokenStream {
    //     let Self {
    //         event_type_ident,
    //         events,
    //         ..
    //     } = self;

    //     let arms = events.keys().map(|name| {
    //         let name_str = name.to_string();
    //         quote! {
    //             #event_type_ident::#name => ::std::write!(#name_str)
    //         }
    //     });

    //     quote! {
    //         #[automatically_derived]
    //         impl ::std::fmt::Display for #event_type_ident {
    //             fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
    //                 match self {
    //                     #( #arms, )*
    //                 }
    //             }
    //         }
    //     }
    // }

    // fn expand_from_event_for_event_type(&self) -> TokenStream {
    //     let Self {
    //         ident,
    //         event_type_ident,
    //         events,
    //     } = self;

    //     let arms = events.keys().map(|name| {
    //         quote! {
    //             #ident::#name(_) => #event_type_ident::#name
    //         }
    //     });

    //     quote! {
    //         #[automatically_derived]
    //         impl ::std::convert::From<&#ident> for #event_type_ident {
    //             fn from(event: &#ident) -> Self {
    //                 match event {
    //                     #( #arms, )*
    //                 }
    //             }
    //         }
    //     }
    // }
}
