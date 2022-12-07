mod rust_type;

use std::fs;

use darling::FromDeriveInput;
use esdl::schema::Param;
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::DeriveInput;

use self::rust_type::{RustTypeDerives, ToEventsType, ToRustType};
use crate::DeriveMacro;

pub struct DeriveAggregate {
    ident: syn::Ident,
    aggregate_trait_ident: syn::Ident,
    commands_ident: syn::Ident,
    events_ident: syn::Ident,
    schema: esdl::schema::Schema,
}

#[derive(FromDeriveInput)]
#[darling(attributes(aggregate))]
struct DeriveAggregateAttrs {
    schema: String,
}

impl DeriveAggregate {
    fn expand_impl_aggregate(&self) -> TokenStream {
        let Self {
            ident,
            aggregate_trait_ident,
            commands_ident,
            events_ident,
            schema,
            ..
        } = self;

        let aggregate_type = &schema.aggregate.name;

        quote! {
            #[automatically_derived]
            impl ::thalo::Aggregate for #ident {
                type Event = #events_ident;
                type Command = #commands_ident;

                fn aggregate_type() -> &'static str {
                    #aggregate_type
                }

                fn new(id: ::std::string::String) -> ::std::result::Result<Self, ::thalo::Error> {
                    <#ident as #aggregate_trait_ident>::new(id)
                }
            }
        }
    }

    fn expand_aggregate_trait(&self) -> TokenStream {
        let Self {
            aggregate_trait_ident,
            schema,
            ..
        } = self;

        let mut apply_methods: Vec<_> = schema.events.keys().collect();
        apply_methods.sort_unstable();
        let apply_methods = apply_methods.into_iter().map(|event_name| {
            let event_ident = format_ident!("{event_name}");
            let apply_method = Self::apply_method(event_name);

            quote! {
                fn #apply_method(&mut self, ctx: ::thalo::Context, event: #event_ident);
            }
        });

        let mut handle_methods: Vec<_> = schema.aggregate.commands.iter().collect();
        handle_methods.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        let handle_methods = handle_methods.into_iter()
            .map(|(command_name, command)| {
                let handle_method = Self::handle_method(command_name);
                let params = command.params.iter().map(|Param { name, ty }| {
                    let name = format_ident!("{name}");
                    let ty = ty.to_rust_type();
                    quote! { #name: #ty }
                });
                let ret_ty = command.events.to_rust_type();

                quote! {
                    fn #handle_method(&self, ctx: &mut ::thalo::Context, #( #params ),*) -> std::result::Result<#ret_ty, thalo::Error>;
                }
            });

        quote! {
            pub trait #aggregate_trait_ident: Sized {
                fn new(id: String) -> std::result::Result<Self, thalo::Error>;
                #( #apply_methods )*
                #( #handle_methods )*
            }
        }
    }

    fn expand_events(&self) -> TokenStream {
        let Self {
            ident,
            aggregate_trait_ident,
            events_ident,
            schema,
            ..
        } = self;

        let ident_str = ident.to_string();

        let variants = schema.events.keys().map(|event_name| {
            let event_ident = format_ident!("{event_name}");
            quote!(#event_ident(#event_ident))
        });

        let events = schema.events.iter().map(|(event_name, event)| {
            let mut derives = event
                .fields
                .derives()
                .into_iter()
                .map(|derive| derive.to_rust_type())
                .peekable();
            let derives = if derives.peek().is_none() {
                quote!()
            } else {
                quote!(#[derive(#( #derives, )*)])
            };

            let event_ident = format_ident!("{event_name}");

            let fields = event.fields.iter().map(|(field_name, field_ty)| {
                let field_ident = format_ident!("{field_name}");
                let ty = field_ty.to_rust_type();
                quote!(pub #field_ident: #ty)
            });

            let apply_method = Self::apply_method(event_name);

            quote! {
                #derives
                pub struct #event_ident {
                    #( #fields, )*
                }

                #[automatically_derived]
                impl ::thalo::Event for #event_ident {
                    type Aggregate = #ident;

                    fn apply(self, state: &mut Self::Aggregate, ctx: ::thalo::Context) {
                        <#ident as #aggregate_trait_ident>::#apply_method(state, ctx, self)
                    }

                    fn event_type() -> &'static str {
                        #event_name
                    }
                }
            }
        });

        quote! {
            #[derive(thalo::Events)]
            #[events(aggregate = #ident_str)]
            pub enum #events_ident {
                #( #variants, )*
            }

            #( #events )*
        }
    }

    fn expand_commands(&self) -> TokenStream {
        let Self {
            ident,
            aggregate_trait_ident,
            commands_ident,
            events_ident,
            schema,
            ..
        } = self;

        let ident_str = ident.to_string();
        let events_str = events_ident.to_string();

        let variants = schema.aggregate.commands.keys().map(|command_name| {
            let command_ident = Self::command_ident(command_name);
            quote!(#command_ident(#command_ident))
        });

        let commands = schema
            .aggregate
            .commands
            .iter()
            .map(|(command_name, command)| {
                let mut derives = command
                    .params
                    .derives()
                    .into_iter()
                    .map(|derive| derive.to_rust_type())
                    .peekable();
                let derives = if derives.peek().is_none() {
                    quote!()
                } else {
                    quote!(#[derive(#( #derives, )*)])
                };

                let command_ident = Self::command_ident(command_name);

                let fields = command.params.iter().map(|param| {
                    let field_ident = format_ident!("{}", param.name);
                    let ty = param.ty.to_rust_type();
                    quote!(pub #field_ident: #ty)
                });

                let handle_method = Self::handle_method(command_name);
                let args = command.params.iter().map(|Param { name, .. }| {
                    let name = format_ident!("{name}");
                    quote! { command.#name }
                });

                let results_ident = format_ident!("result");
                let results_to_vec = command.events.to_events_type(&results_ident, events_ident);

                quote! {
                    #derives
                    pub struct #command_ident {
                        #( #fields, )*
                    }

                    #[automatically_derived]
                    impl ::thalo::Command<#command_ident, ::thalo::Error> for #ident {
                        fn handle(&self, ctx: &mut ::thalo::Context, command: #command_ident) -> Result<Vec<#events_ident>, ::thalo::Error> {
                            <#ident as #aggregate_trait_ident>::#handle_method(self, ctx, #( #args ),*).map(|#results_ident| {
                                #results_to_vec
                            })
                        }
                    }
                }
            });

        quote! {
            #[derive(thalo::Commands)]
            #[commands(aggregate = #ident_str, events = #events_str)]
            pub enum #commands_ident {
                #( #variants, )*
            }

            #( #commands )*
        }
    }

    fn apply_method(event_name: &str) -> syn::Ident {
        format_ident!("apply_{}", event_name.to_snake_case())
    }

    fn handle_method(command_name: &str) -> syn::Ident {
        format_ident!("handle_{}", command_name.to_snake_case())
    }

    fn command_ident(command_name: &str) -> syn::Ident {
        format_ident!("{}", command_name.to_upper_camel_case())
    }
}

impl DeriveMacro for DeriveAggregate {
    fn expand(&self) -> TokenStream {
        let impl_aggregate_expanded = self.expand_impl_aggregate();
        let aggregate_trait_expanded = self.expand_aggregate_trait();
        let events_expanded = self.expand_events();
        let commands_expanded = self.expand_commands();

        quote! {
            #impl_aggregate_expanded
            #aggregate_trait_expanded
            #events_expanded
            #commands_expanded
        }
    }

    fn parse_input(input: DeriveInput) -> syn::Result<Self> {
        let attrs = DeriveAggregateAttrs::from_derive_input(&input)?;
        let schema_content = fs::read_to_string(attrs.schema).map_err(|err| {
            syn::Error::new(input.span(), format!("failed to read schema file: {err}"))
        })?;
        let schema = esdl::parse(&schema_content).map_err(|err| {
            syn::Error::new(input.span(), format!("failed to parse schema file: {err}"))
        })?;

        let ident = input.ident;
        let aggregate_trait_ident = format_ident!("{ident}Aggregate");
        let commands_ident = format_ident!("{ident}Command");
        let events_ident = format_ident!("{ident}Event");

        Ok(DeriveAggregate {
            ident,
            aggregate_trait_ident,
            commands_ident,
            events_ident,
            schema,
        })
    }
}
