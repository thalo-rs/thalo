use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    ItemEnum,
};

#[proc_macro_derive(Events)]
pub fn events(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as DeriveEvents)
        .expand()
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

struct DeriveEvents {
    ident: syn::Ident,
    event_ident: syn::Ident,
    events: HashMap<syn::Ident, Vec<syn::Path>>,
}

impl Parse for DeriveEvents {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_enum: ItemEnum = input.parse()?;
        let event_ident = format_ident!("Event");
        let events = item_enum
            .variants
            .into_iter()
            .map(|variant| {
                let event = variant.ident;
                let versions = match variant.fields {
                    syn::Fields::Named(_) => {
                        return Err(syn::Error::new(
                            variant.fields.span(),
                            format!("event versions must be unnamed fields"),
                        ));
                    }
                    syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) => unnamed
                        .into_iter()
                        .map(|field| match field.ty {
                            syn::Type::Path(type_path) => Ok(type_path.path),
                            _ => Err(syn::Error::new(field.ty.span(), "expected type path")),
                        })
                        .collect::<Result<_, _>>()?,
                    syn::Fields::Unit => {
                        return Err(syn::Error::new(
                            variant.fields.span(),
                            format!("events must have at least one version"),
                        ));
                    }
                };
                Ok((event, versions))
            })
            .collect::<Result<_, _>>()?;

        Ok(DeriveEvents {
            ident: item_enum.ident,
            event_ident,
            events,
        })
    }
}

impl DeriveEvents {
    fn expand(self) -> syn::Result<TokenStream> {
        let ident = &self.ident;
        let event_ident = &self.event_ident;

        let event_variants: Vec<_> = self
            .events
            .iter()
            .map(|(event, versions)| {
                let latest_version = versions.last().ok_or_else(|| {
                    syn::Error::new(event.span(), "expected at least one version")
                })?;

                Ok::<_, syn::Error>(quote! {
                    #event(#latest_version)
                })
            })
            .collect::<Result<_, _>>()?;

        let versioned_event_variants: Vec<_> = self.events.iter().flat_map(|(event, versions)| {
            versions.iter().enumerate().map(move |(version, path)| {
                let variant_name = format_ident!("{}V{}", event, version + 1);
                quote! {
                    #variant_name(#path)
                }
            })
        }).collect();

        let versioned_event_ref_variants = self.events.iter().flat_map(|(event, versions)| {
            versions.iter().enumerate().map(move |(version, path)| {
                let variant_name = format_ident!("{}V{}", event, version + 1);
                quote! {
                    #variant_name(&'a #path)
                }
            })
        });

        let upcast_arms = self.events.iter().flat_map(|(event, versions)| {
            versions.iter().enumerate().map(move |(version, _path)| {
                let variant_name = format_ident!("{}V{}", event, version + 1);
                let from = versions.windows(2).skip(version).fold(quote! { event }, |acc, window| {
                    let from = &window[0];
                    let to = &window[1];

                    quote! {
                        <#to as ::std::convert::From<#from>>::from(#acc)
                    }
                });
            
                quote! {
                    VersionedEvent::#variant_name(event) => #event_ident::#event(#from)
                }
            })
        });

        let from_event_arms = self
            .events
            .iter()
            .map(|(event, versions)| {
                let versioned_event_ref_variant = format_ident!("{}V{}", event, versions.len());

                quote! {
                    #event_ident::#event(event) => VersionedEventRef::#versioned_event_ref_variant(event)                    
                }
            });

        Ok(quote! {
            pub enum #event_ident {
                #( #event_variants, )*
            }

            const _: () = {
                impl ::thalo::Events for #ident {
                    type Event = #event_ident;
                }

                impl ::serde::Serialize for Event {
                    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
                    where
                        S: ::serde::Serializer,
                    {
                        let versioned_event: VersionedEventRef = self.into();
                        <VersionedEventRef as ::serde::Serialize>::serialize(&versioned_event, serializer)
                    }
                }

                impl<'de> ::serde::Deserialize<'de> for Event {
                    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
                    where
                        D: ::serde::Deserializer<'de>,
                    {
                        let versioned_event: VersionedEvent =
                            <VersionedEvent as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                        ::std::result::Result::Ok(versioned_event.upcast())
                    }
                }

                #[derive(::serde::Deserialize)]
                #[serde(tag = "event", content = "payload")]
                enum VersionedEvent {
                    #( #versioned_event_variants, )*
                }

                #[derive(::serde::Serialize)]
                #[serde(tag = "event", content = "payload")]
                enum VersionedEventRef<'a> {
                    #( #versioned_event_ref_variants, )*
                }

                impl VersionedEvent {
                    fn upcast(self) -> Event {
                        match self {
                            #( #upcast_arms, )*
                        }
                    }
                }

                impl<'a> ::std::convert::From<&'a Event> for VersionedEventRef<'a> {
                    fn from(event: &'a Event) -> Self {
                        match event {
                            #( #from_event_arms, )*
                        }
                    }
                }
            };
        })
    }
}
