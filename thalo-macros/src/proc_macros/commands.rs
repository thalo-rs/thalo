use heck::{CamelCase, ShoutySnekCase};
use proc_macro2::{Literal, TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::spanned::Spanned;

pub struct Commands {
    command_ident: syn::Ident,
    event_ident: syn::Ident,
    has_tarpc_arg: bool,
    ident: syn::Ident,
    input: syn::ItemImpl,
    methods: Vec<Method>,
    server_ident: syn::Ident,
    service_ident: syn::Ident,
}

struct Method {
    args: Vec<Arg>,
    docs: Vec<Literal>,
    ident: syn::Ident,
    is_opt: bool,
    is_vec: bool,
}

struct Arg {
    ident: syn::Ident,
    ty: syn::Type,
}

impl Commands {
    fn expand_command_enum(&self) -> syn::Result<TokenStream> {
        let Self {
            command_ident,
            ident,
            methods,
            ..
        } = self;

        let ident = ident.to_string();

        let variants = methods.iter().map(|method| {
            let docs = &method.docs;

            let variant_ident = format_ident!("{}", method.ident.to_string().to_camel_case());
            let variant_ident_upper = method.ident.to_string().TO_SHOUTY_SNEK_CASE();

            let fields = method.args.iter().map(|arg| {
                let ident = &arg.ident;
                let ty = &arg.ty;
                quote!(#ident: #ty)
            });

            quote!(
                #(#[doc = #docs])*
                #[serde(rename = #variant_ident_upper)]
                #variant_ident {
                    #( #fields, )*
                }
            )
        });

        Ok(quote!(
            #[derive(Clone, Debug, PartialEq, ::thalo::Command, ::thalo::StreamTopic, ::thalo::CommandMessage, ::serde::Deserialize, ::serde::Serialize)]
            #[aggregate = #ident]
            pub enum #command_ident {
                #( #variants, )*
            }
        ))
    }

    fn expand_impl_aggregate_command_handler(&self) -> syn::Result<TokenStream> {
        let Self {
            command_ident,
            event_ident,
            ident,
            methods,
            ..
        } = self;

        let matches = methods.iter()
            .map(|method| {
                let method_ident = &method.ident;
                let variant_ident =
                    format_ident!("{}", method.ident.to_string().to_camel_case());

                let fields: Vec<_> = method.args.iter().map(|arg| {
                    &arg.ident
                }).collect();

                if method.is_vec {
                    quote!(
                        #command_ident::#variant_ident { #( #fields, )* } => Ok(self.#method_ident(#( #fields ),*)?.into_iter().map(|event| event.into()).collect())
                    )
                } else if method.is_opt {
                    quote!(
                        #command_ident::#variant_ident { #( #fields, )* } => self.#method_ident(#( #fields ),*).map(|event_opt| event_opt.map(|event| vec![event.into()]).unwrap_or_default())
                    )
                } else {
                    quote!(
                        #command_ident::#variant_ident { #( #fields, )* } => self.#method_ident(#( #fields ),*).map(|event| vec![event.into()])
                    )
                }
            });

        Ok(quote!(
            impl ::thalo::AggregateCommandHandler for #ident {
                type Command = #command_ident;
                type Event = #event_ident;

                fn execute(&self, command: Self::Command) -> Result<Vec<Self::Event>, ::thalo::Error> {
                    match command {
                        #( #matches, )*
                    }
                }
            }
        ))
    }

    fn expand_tarpc_service(&self) -> TokenStream {
        let Self {
            command_ident,
            event_ident,
            ident,
            methods,
            server_ident,
            service_ident,
            ..
        } = self;

        let service_methods = methods.iter().map(|method| {
            let method_ident = &method.ident;
            let method_args = method.args.iter().map(|arg| {
                let arg_ident = &arg.ident;
                let arg_type = &arg.ty;
                quote!(#arg_ident: #arg_type)
            });

            quote!(
                async fn #method_ident(
                    id: ::std::string::String,
                    #( #method_args, )*
                ) -> ::std::result::Result<::std::vec::Vec<::thalo::EventEnvelope<#event_ident>>, ::std::string::String>;
            )
        });

        let server_methods = methods.iter().map(|method| {
            let method_ident = &method.ident;
            let method_args = method.args.iter().map(|arg| {
                let arg_ident = &arg.ident;
                let arg_type = &arg.ty;
                quote!(#arg_ident: #arg_type)
            });
            let method_command_ident =
                format_ident!("{}", method_ident.to_string().to_camel_case());
            let method_command_args = method.args.iter().map(|arg| &arg.ident);

            quote!(
                async fn #method_ident(
                    self,
                    _: ::tarpc::context::Context,
                    id: ::std::string::String,
                    #( #method_args, )*
                ) -> ::std::result::Result<
                    ::std::vec::Vec<::thalo::EventEnvelope<#event_ident>>,
                    ::std::string::String,
                > {
                    <#ident as ::thalo::postgres::SendCommand<#command_ident>>::send_command(
                        &id,
                        #command_ident::#method_command_ident {
                            #( #method_command_args ),*
                        },
                    )
                    .await
                    .map_err(|err| ::serde_json::to_string(&match err {
                        ::thalo::Error::Invariant(code, msg) => {
                            ::serde_json::json!({
                                "code": code,
                                "message": msg,
                            })
                        }
                        _ => {
                            ::serde_json::json!({
                                "code": "THALO_ERROR",
                                "message": err.to_string(),
                            })
                        }
                    }).unwrap_or_default())
                }
            )
        });

        quote!(
            #[::tarpc::service]
            pub trait #service_ident {
                #( #service_methods )*
            }

            #[derive(Clone, Copy)]
            pub struct #server_ident;

            #[::tarpc::server]
            impl #service_ident for #server_ident {
                #( #server_methods )*
            }
        )
    }
}

impl Commands {
    pub fn new(
        args: syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>,
        input: syn::ItemImpl,
    ) -> syn::Result<Self> {
        let has_tarpc_arg = args.into_iter().any(|arg| arg == "tarpc");
        let ident = match &*input.self_ty {
            syn::Type::Path(type_path) => type_path.path.get_ident().unwrap().clone(),
            _ => {
                return Err(syn::Error::new(
                    input.impl_token.span,
                    "impl must be on a struct",
                ))
            }
        };

        let event_ident = format_ident!("{}Event", ident);
        let command_ident = format_ident!("{}Command", ident);
        let server_ident = format_ident!("{}Server", ident);
        let service_ident = format_ident!("{}Service", ident);

        let methods = input
            .items
            .clone()
            .into_iter()
            .map(|item| match item {
                syn::ImplItem::Method(method) => Result::<_, syn::Error>::Ok(method),
                _ => Err(syn::Error::new(
                    item.span(),
                    "unexpected item: only methods are allowed in aggregate_events",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|method| {
                let docs: Vec<_> = method
                    .attrs
                    .into_iter()
                    .filter_map(|attr| {
                        if !matches!(attr.style, syn::AttrStyle::Outer) {
                            return None;
                        }

                        if attr.path.segments.first()?.ident != "doc" {
                            return None;
                        }

                        attr.tokens.into_iter().nth(1).and_then(|doc| match doc {
                            TokenTree::Literal(lit) => Some(lit),
                            _ => None,
                        })
                    })
                    .collect();

                let mut inputs = method.sig.inputs.into_iter();
                let mut is_vec = false;
                let mut is_opt = false;

                let self_input = inputs.next().ok_or_else(|| {
                    syn::Error::new(method.sig.ident.span(), "method must take &self")
                })?;
                match self_input {
                    syn::FnArg::Receiver(receiver) => {
                        if let Some(mutability) = receiver.mutability {
                            return Err(syn::Error::new(
                                mutability.span(),
                                "self cannot be mut for aggregate commands",
                            ));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new(
                            method.sig.ident.span(),
                            "method must take &mut self",
                        ));
                    }
                }

                let args: Vec<_> = inputs
                    .map(|arg| {
                        let arg = match arg {
                            syn::FnArg::Typed(arg) => arg,
                            _ => unreachable!("methods cannot take self more than once"),
                        };
                        let ident = match &*arg.pat {
                            syn::Pat::Ident(ident_pat) => format_ident!(
                                "{}",
                                ident_pat.ident.to_string().trim_start_matches('_')
                            ),
                            _ => {
                                return Err(syn::Error::new(
                                    arg.span(),
                                    "unsupported argument type",
                                ))
                            }
                        };
                        let ty = *arg.ty;

                        Ok(Arg { ident, ty })
                    })
                    .collect::<Result<_, _>>()?;

                let err_ty = match &method.sig.output {
                    syn::ReturnType::Type(_, ty) => match &**ty {
                        syn::Type::Path(ty_path) => {
                            let mut segments = ty_path.path.segments.iter();
                            segments.next().and_then(|segment| {
                                if segments.next().is_some() {
                                    return None;
                                }
                                if segment.ident != "Result" {
                                    return None;
                                }
                                let arguments = match &segment.arguments {
                                    syn::PathArguments::AngleBracketed(arguments) => arguments,
                                    _ => return None,
                                };
                                let mut args = arguments.args.iter();
                                let first_argument = args.next()?;
                                let first_argument_string = quote!(#first_argument).to_string();
                                is_vec = first_argument_string.starts_with("Vec <");
                                is_opt = first_argument_string.starts_with("Option <");
                                match args.next()? {
                                    syn::GenericArgument::Type(ty) => Some(ty),
                                    _ => None,
                                }
                            })
                        }
                        _ => None,
                    },
                    _ => None,
                };
                match err_ty {
                    Some(err_ty) => {
                        let err_ty_string = quote!(#err_ty).to_string().replace(' ', "");
                        if err_ty_string != "Error"
                            && err_ty_string != "thalo::Error"
                            && err_ty_string != "::thalo::Error"
                        {
                            return Err(syn::Error::new(
                                err_ty.span(),
                                format!(
                                    "method must return `Result<Vec<{}>, thalo::Error>`",
                                    event_ident.to_string(),
                                ),
                            ));
                        }
                    }
                    None => {
                        return Err(syn::Error::new(
                            method.sig.output.span(),
                            format!(
                                "method must return `Result<Vec<{}>, thalo::Error>`",
                                event_ident.to_string(),
                            ),
                        ))
                    }
                }

                Ok(Method {
                    args,
                    docs,
                    ident: method.sig.ident,
                    is_opt,
                    is_vec,
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(Commands {
            command_ident,
            event_ident,
            has_tarpc_arg,
            ident,
            input,
            methods,
            server_ident,
            service_ident,
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let input = &self.input;

        let expanded_input = quote!(#input);
        let expanded_events_enum = self.expand_command_enum()?;
        let expanded_impl_aggregate_command = self.expand_impl_aggregate_command_handler()?;
        let expanded_tarpc_service = if self.has_tarpc_arg {
            self.expand_tarpc_service()
        } else {
            quote!()
        };

        Ok(TokenStream::from_iter([
            expanded_input,
            expanded_events_enum,
            expanded_impl_aggregate_command,
            expanded_tarpc_service,
        ]))
    }
}
