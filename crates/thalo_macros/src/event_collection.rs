use proc_macro2::TokenStream;
use quote::{quote};
use syn::{
    spanned::Spanned,
    DeriveInput,
};

use crate::DeriveMacro;

pub struct DeriveEventCollection {
    ident: syn::Ident,
    variants: Vec<Variant>,
}

struct Variant {
    variant_ident: syn::Ident,
    event_path: syn::Path,
}

impl DeriveEventCollection {
    fn expand_impl_event_collection(&self) -> TokenStream {
        let Self {
            ident,
            variants,
        } = self;

        let categories = variants.iter().map(|Variant { event_path, .. }| {
            quote! {
                <<#event_path as ::thalo::Event>::Aggregate as ::thalo::Aggregate>::aggregate_type()
            }
        });

        let deserialize_events = variants.iter().map(|Variant { variant_ident, event_path }| {
            quote! {
                if entity_name
                    == <<#event_path as ::thalo::Event>::Aggregate as ::thalo::Aggregate>::aggregate_type()
                    && event_type == <#event_path as ::thalo::Event>::event_type()
                {
                    let message = message
                        .deserialize_data::<#event_path>()?
                        .map_data(#ident::#variant_ident);
                    return Ok(Some(message));
                }
            }
        });
     
        quote! {
            #[automatically_derived]
            impl ::thalo::consumer::EventCollection for #ident {
                fn entity_names() -> ::std::collections::HashSet<&'static str> {
                    ::std::collections::HashSet::from([
                        #( #categories ),*
                    ])
                }

                fn deserialize_event(
                    message: ::messagedb::message::GenericMessage,
                ) -> ::std::result::Result<
                    ::std::option::Option<::messagedb::message::Message<Self>>,
                    ::thalo::serde_json::Error,
                > {
                    let entity_name = &message.stream_name.category.entity_name;
                    let event_type = &message.msg_type;
                    
                    #( #deserialize_events )*

                    Ok(None)
                }
            }
        }
    }
}

impl DeriveMacro for DeriveEventCollection {
    fn expand(&self) -> TokenStream {
        self.expand_impl_event_collection()
    }

    fn parse_input(input: DeriveInput) -> syn::Result<Self> {
        let span = input.span();
        let variants = match input.data {
            syn::Data::Enum(syn::DataEnum { variants, .. }) => {
                variants.into_iter().map(|variant| {
                    match variant.fields {
                        syn::Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => { 
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
                            
                            Ok(Variant { variant_ident: variant.ident, event_path: ty })
                        }
                        syn::Fields::Unnamed(fields_unnamed) => {
                            Err(syn::Error::new(
                                fields_unnamed.span(),
                                "DeriveEventCollection variants must contain only one unnamed field. Eg. `Incremented(IncrementedEvent)`",
                            ))
                        }
                        syn::Fields::Named(fields_named) => {
                            Err(syn::Error::new(
                                fields_named.span(),
                                "DeriveEventCollection variants cannot have named fields. Variants must use unnamed fields. Eg. `Incremented(IncrementedEvent)`",
                            ))
                        }
                        syn::Fields::Unit => {
                            Err(syn::Error::new(
                                variant.fields.span(),
                                "DeriveEventCollection expected an unamed field. Eg. `Incremented(IncrementedEvent)`",
                            ))
                        }
                    }
                }).collect::<Result<Vec<_>, _>>()?
            },
            _ => {
                return Err(syn::Error::new(
                    span,
                    "DeriveEventCollection can only be used with enums",
                ));
            }
        };

        Ok(DeriveEventCollection {
            ident: input.ident,
            variants,
        })
    }
}
