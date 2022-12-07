use darling::{FromDeriveInput, FromVariant};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{quote};
use syn::{
    spanned::Spanned,
    DeriveInput,
};

use crate::DeriveMacro;

pub struct DeriveCommands {
    aggregate_ident: syn::Path,
    events_ident: syn::Path,
    ident: syn::Ident,
    variants: Vec<(String, syn::Ident, syn::Path)>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(commands))]
struct DeriveCommandsAttrs {
    aggregate: syn::Path,
    events: syn::Path,
}

#[derive(Default, FromVariant)]
#[darling(default)]
struct DeriveCommandsVariantAttrs {
    command_name: Option<String>,
}

impl DeriveCommands {
    fn expand_impl_commands(&self) -> TokenStream {
        let Self {
            aggregate_ident,
            events_ident,
            ident,
            variants,
        } = self;
        
        let handle_variants = variants.iter().map(|(command_name, command_ident, command_path)| {
            quote! {
                #command_name => {
                    let command: #command_path =
                        ::thalo::serde_json::from_slice(&payload).map_err(|err| ::thalo::ErrorKind::DeserializeCommand(err.to_string()))?;
                    #[allow(unreachable_code, clippy::diverging_sub_expression)]
                    if false { let _ = #ident::#command_ident(unreachable!()); }
                    <#aggregate_ident as ::thalo::Command<#command_path, _>>::handle(state, ctx, command)
                }
            }
        });

        quote! {
            #[automatically_derived]
            impl ::thalo::Commands for #ident {
                type Aggregate = #aggregate_ident;

                fn handle(
                    state: &#aggregate_ident,
                    ctx: &mut ::thalo::Context,
                    command_name: &str,
                    payload: ::std::vec::Vec<u8>,
                ) -> ::std::result::Result<
                    ::std::vec::Vec<#events_ident>,
                    ::thalo::Error
                > {
                    match command_name {
                        #( #handle_variants )*
                        _ => ::std::result::Result::Err(::thalo::ErrorKind::UnknownCommand.into()),
                    }
                }
            }
        }
    }
}

impl DeriveMacro for DeriveCommands {
    fn expand(&self) -> proc_macro2::TokenStream {
        self.expand_impl_commands()
    }

    fn parse_input(input: DeriveInput) -> syn::Result<Self> {
        let span = input.span();
        let attrs = DeriveCommandsAttrs::from_derive_input(&input)?;
        let variants = match input.data {
            syn::Data::Enum(syn::DataEnum { variants, .. }) => {
                variants.into_iter().map(|variant| {
                    let DeriveCommandsVariantAttrs { command_name } = DeriveCommandsVariantAttrs::from_variant(&variant)?;

                    match variant.fields {
                        syn::Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => { 
                            let name = command_name.unwrap_or_else(|| variant.ident.to_string().to_snake_case());
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
                                        "Variant should be a path to an command struct. Eg. `Increment(IncrementCommand)`",
                                    ));
                                }
                            };
                            
                            Ok((name, variant.ident, ty))
                        }
                        syn::Fields::Unnamed(fields_unnamed) => {
                            Err(syn::Error::new(
                                fields_unnamed.span(),
                                "DeriveCommands variants must contain only one unnamed field. Eg. `Increment(IncrementCommand)`",
                            ))
                        }
                        syn::Fields::Named(fields_named) => {
                            Err(syn::Error::new(
                                fields_named.span(),
                                "DeriveCommands variants cannot have named fields. Variants must use unnamed fields. Eg. `Increment(IncrementCommand)`",
                            ))
                        }
                        syn::Fields::Unit => {
                            Err(syn::Error::new(
                                variant.fields.span(),
                                "DeriveCommands expected an unamed field. Eg. `Increment(IncrementCommand)`",
                            ))
                        }
                    }
                }).collect::<Result<Vec<_>, _>>()?
            },
            _ => {
                return Err(syn::Error::new(
                    span,
                    "DeriveCommands can only be used with enums",
                ));
            }
        };

        Ok(DeriveCommands {
            aggregate_ident: attrs.aggregate,
            events_ident: attrs.events,
            ident: input.ident,
            variants,
        })
    }
}
