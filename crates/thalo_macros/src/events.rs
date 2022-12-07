use darling::{FromDeriveInput, FromVariant};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{quote};
use syn::{
    spanned::Spanned,
    DeriveInput,
};

use crate::DeriveMacro;

pub struct DeriveEvents {
    aggregate_ident: syn::Path,
    ident: syn::Ident,
    variants: Vec<(String, syn::Ident, syn::Path)>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(events))]
struct DeriveEventsAttrs {
    aggregate: syn::Path,
}

#[derive(Default, FromVariant)]
#[darling(default)]
struct DeriveEventsVariantAttrs {
    event_name: Option<String>,
}

impl DeriveEvents {
    fn expand_impl_events(&self) -> TokenStream {
        let Self {
            aggregate_ident,
            ident,
            variants,
        } = self;
        
        let apply_variants = variants.iter().map(|(event_name, _, event_path)| {
            quote! {
                #event_name => {
                    let event: #event_path =
                        ::thalo::serde_json::from_slice(&payload)
                            .map_err(|err| ::thalo::ErrorKind::DeserializeEvent(err.to_string()))?;
                        <#event_path as ::thalo::Event>::apply(event, state, ctx);
                }
            }
        });

        let event_types = variants.iter().map(|(_, event_ident, event_path)| {
            quote! {
                #ident::#event_ident(_) => <#event_path as ::thalo::Event>::event_type()
            }
        });

        let payloads = variants.iter().map(|(_, event_ident, _)| {
            quote! {
                #ident::#event_ident(event) => ::thalo::serde_json::to_vec(&event)
            }
        });

        quote! {
            #[automatically_derived]
            impl ::thalo::Events for #ident {
                type Aggregate = #aggregate_ident;

                fn apply(
                    state: &mut #aggregate_ident,
                    ctx: ::thalo::Context,
                    event_type: &str,
                    payload: std::vec::Vec<u8>,
                ) -> ::std::result::Result<(), ::thalo::Error> {
                    match event_type {
                        #( #apply_variants )*
                        _ => return ::std::result::Result::Err(::thalo::ErrorKind::UnknownEvent.into()),
                    }
                    Ok(())
                }

                fn event_type(&self) -> &'static str {
                    match self {
                        #( #event_types, )*
                    }
                }

                fn payload(self) -> ::std::result::Result<std::vec::Vec<u8>, ::thalo::Error> {
                    let payload = match self {
                        #( #payloads, )*
                    }
                    .map_err(|err| ::thalo::ErrorKind::SerializeEvent(err.to_string()))?;

                    ::std::result::Result::Ok(payload)
                }
            }
        }
    }
}

impl DeriveMacro for DeriveEvents {
    fn expand(&self) -> TokenStream {
        self.expand_impl_events()
    }

    fn parse_input(input: DeriveInput) -> syn::Result<Self> {
        let span = input.span();
        let attrs = DeriveEventsAttrs::from_derive_input(&input)?;
        let variants = match input.data {
            syn::Data::Enum(syn::DataEnum { variants, .. }) => {
                variants.into_iter().map(|variant| {
                    let DeriveEventsVariantAttrs { event_name } = DeriveEventsVariantAttrs::from_variant(&variant)?;

                    match variant.fields {
                        syn::Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => { 
                            let name = event_name.unwrap_or_else(|| variant.ident.to_string().to_upper_camel_case());
                            let field = fields_unnamed
                                .unnamed
                                .into_iter()
                                .next()
                                .expect("length already checked");
                            let ty = match field.ty {
                                syn::Type::Path(type_path) => {
                                    type_path.path
                                }
                                ty => {
                                    return Err(syn::Error::new(
                                        ty.span(),
                                        "Variant should be a path to an event struct. Eg. `Incremented(IncrementedEvent)`",
                                    ));
                                }
                            };
                            
                            Ok((name, variant.ident, ty))
                        }
                        syn::Fields::Unnamed(fields_unnamed) => {
                            Err(syn::Error::new(
                                fields_unnamed.span(),
                                "DeriveEvents variants must contain only one unnamed field. Eg. `Incremented(IncrementedEvent)`",
                            ))
                        }
                        syn::Fields::Named(fields_named) => {
                            Err(syn::Error::new(
                                fields_named.span(),
                                "DeriveEvents variants cannot have named fields. Variants must use unnamed fields. Eg. `Incremented(IncrementedEvent)`",
                            ))
                        }
                        syn::Fields::Unit => {
                            Err(syn::Error::new(
                                variant.fields.span(),
                                "DeriveEvents expected an unamed field. Eg. `Incremented(IncrementedEvent)`",
                            ))
                        }
                    }
                }).collect::<Result<Vec<_>, _>>()?
            },
            _ => {
                return Err(syn::Error::new(
                    span,
                    "DeriveEvents can only be used with enums",
                ));
            }
        };

        Ok(DeriveEvents {
            aggregate_ident: attrs.aggregate,
            ident: input.ident,
            variants,
        })
    }
}
