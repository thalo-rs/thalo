use heck::CamelCase;
use heck::ShoutySnekCase;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::{format_ident, quote};
use syn::spanned::Spanned;

pub struct AggregateEvents {
    event_ident: syn::Ident,
    ident: syn::Ident,
    input: syn::ItemImpl,
    methods: Vec<Method>,
}

struct Method {
    args: Vec<Arg>,
    docs: Vec<Literal>,
    ident: syn::Ident,
    struct_ident: syn::Ident,
    variant_ident: syn::Ident,
}

struct Arg {
    attrs: Vec<syn::Attribute>,
    ident: syn::Ident,
    ty: syn::Type,
}

impl AggregateEvents {
    fn expand_event_enum(&self) -> syn::Result<TokenStream> {
        let Self {
            event_ident,
            ident,
            methods,
            ..
        } = self;

        let ident = ident.to_string();

        let (structs, variants): (Vec<_>, Vec<_>) = methods
            .iter()
            .map(|method| {
                let docs = &method.docs;

                let struct_ident =
                    format_ident!("{}Event", method.ident.to_string().to_camel_case());
                let variant_ident = format_ident!("{}", method.ident.to_string().to_camel_case());
                let variant_ident_upper = method.ident.to_string().TO_SHOUTY_SNEK_CASE();

                let fields = method.args.iter().map(|arg| {
                    let attrs = &arg.attrs;
                    let ident = &arg.ident;
                    let ty = &arg.ty;
                    quote!(#( #attrs )* pub #ident: #ty)
                });

                (
                    quote!(
                        #[derive(Clone, Debug, PartialEq, ::thalo::EventIdentity, ::serde::Deserialize, ::serde::Serialize)]
                        #(#[doc = #docs])*
                        pub struct #struct_ident {
                            #( #fields, )*
                        }

                        impl ::std::convert::From<#struct_ident> for #event_ident {
                            fn from(event: #struct_ident) -> Self {
                                #event_ident::#variant_ident(event)
                            }
                        }
                    ),
                    quote!(
                        #[serde(rename = #variant_ident_upper)]
                        #variant_ident(#struct_ident)
                    ),
                )
            })
            .unzip();

        Ok(quote!(
            #(
                #structs
            )*

            #[derive(Clone, Debug, PartialEq, ::thalo::Event, ::thalo::StreamTopic, ::serde::Deserialize, ::serde::Serialize)]
            #[aggregate = #ident]
            pub enum #event_ident {
                #( #variants, )*
            }
        ))
    }

    fn expand_impl_aggregate_event_handler(&self) -> syn::Result<TokenStream> {
        let Self {
            event_ident,
            ident,
            methods,
            ..
        } = self;

        let matches = methods.iter()
            .map(|method| {
                let method_ident = &method.ident;
                let struct_ident = &method.struct_ident;
                let variant_ident = &method.variant_ident;

                let fields: Vec<_> = method.args.iter().map(|arg| {
                    &arg.ident
                }).collect();

                quote!(
                    #event_ident::#variant_ident(#struct_ident { #( #fields, )* }) => self.#method_ident(#( #fields ),*)
                )
            });

        Ok(quote!(
            impl ::thalo::AggregateEventHandler for #ident {
                type Event = #event_ident;

                fn apply(&mut self, event: Self::Event) {
                    match event {
                        #( #matches, )*
                    }
                }
            }
        ))
    }

    fn expand_impl_event_view(&self) -> TokenStream {
        let Self {
            event_ident,
            methods,
            ..
        } = self;

        let impls = methods.iter().map(|method| {
            let struct_ident = &method.struct_ident;
            let variant_ident = &method.variant_ident;

            quote!(
                impl ::thalo::EventView<#struct_ident> for ::std::vec::Vec<::thalo::EventEnvelope<#event_ident>> {
                    fn view(&self) -> ::std::result::Result<&#struct_ident, ::thalo::Error> {
                        self.view_opt()
                            .ok_or_else(|| ::thalo::Error::EventMissing(<#struct_ident as ::thalo::EventIdentity>::event_type()))
                    }
                    
                    fn view_opt(&self) -> ::std::option::Option<&#struct_ident> {
                        self.iter().find_map(|event| match &event.event {
                            #event_ident::#variant_ident(ev) => Some(ev),
                            _ => None,
                        })
                    }
                }
            )
        });

        quote!(
            #( #impls )*
        )
    }
}

impl AggregateEvents {
    pub fn new(mut input: syn::ItemImpl) -> syn::Result<Self> {
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

                if !matches!(method.sig.output, syn::ReturnType::Default) {
                    return Err(syn::Error::new(
                        method.sig.ident.span(),
                        "method cannot have a return type",
                    ));
                }

                let method_ident = method.sig.ident;
                let struct_ident =
                    format_ident!("{}Event", method_ident.to_string().to_camel_case());
                let variant_ident = format_ident!("{}", method_ident.to_string().to_camel_case());

                let mut inputs = method.sig.inputs.into_iter();

                let self_input = inputs.next().ok_or_else(|| {
                    syn::Error::new(method_ident.span(), "method must take &mut self")
                })?;
                if !matches!(self_input, syn::FnArg::Receiver(_)) {
                    return Err(syn::Error::new(
                        method_ident.span(),
                        "method must take &mut self",
                    ));
                }

                let args: Vec<_> = inputs
                    .map(|arg| {
                        let arg = match arg {
                            syn::FnArg::Typed(arg) => arg,
                            _ => unreachable!("methods cannot take self more than once"),
                        };
                        let attrs = arg.attrs.clone();
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

                        Ok(Arg { attrs, ident, ty })
                    })
                    .collect::<Result<_, _>>()?;

                Ok(Method {
                    args,
                    docs,
                    ident: method_ident,
                    struct_ident,
                    variant_ident,
                })
            })
            .collect::<Result<_, _>>()?;

        for item in &mut input.items {
            let method = match item {
                syn::ImplItem::Method(method) => method,
                _ => return Err(syn::Error::new(
                    item.span(),
                    "unexpected item: only methods are allowed in aggregate_events",
                )),
            };

            for arg in &mut method.sig.inputs {
                let arg = match arg {
                    syn::FnArg::Typed(arg) => {arg},
                    _ => continue,
                };
                arg.attrs = vec![];
            }
        }

        Ok(AggregateEvents {
            event_ident,
            ident,
            input,
            methods,
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let input = &self.input;

        let expanded_input = quote!(#input);
        let expanded_event_enum = self.expand_event_enum()?;
        let expanded_impl_aggregate_state_mutator = self.expand_impl_aggregate_event_handler()?;
        let expanded_impl_event_view = self.expand_impl_event_view();

        Ok(TokenStream::from_iter([
            expanded_input,
            expanded_event_enum,
            expanded_impl_aggregate_state_mutator,
            expanded_impl_event_view,
        ]))
    }
}
