use heck::ShoutySnekCase;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;

pub struct Event {
    aggregate: TokenStream,
    ident: syn::Ident,
    variants: syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
}

impl Event {
    fn expand_impl_event(&self) -> syn::Result<TokenStream> {
        let Self {
            aggregate,
            ident,
            variants,
        } = self;

        let variant_idents = variants.iter().map(|variant| &variant.ident);
        let variant_strings = variants
            .iter()
            .map(|variant| variant.ident.to_string().TO_SHOUTY_SNEK_CASE());

        Ok(quote!(
            impl ::awto_es::Event for #ident {
                type Aggregate = #aggregate;

                fn event_type(&self) -> &'static str {
                    match self {
                        #( Self::#variant_idents { .. } => #variant_strings, )*
                    }
                }

                fn aggregate_event<'a>(&self, aggregate_id: &'a str) -> Result<::awto_es::AggregateEvent<'a>, ::awto_es::Error> {
                    Ok(::awto_es::AggregateEvent {
                        aggregate_type: <Self::Aggregate as ::awto_es::AggregateType>::aggregate_type(),
                        aggregate_id,
                        event_type: <Self as ::awto_es::Event>::event_type(self),
                        event_data: ::serde_json::to_value(&self).map_err(|err| {
                            ::awto_es::Error::new(
                                ::awto_es::ErrorKind::SerializeError,
                                "could not serialize event",
                            )
                        })?,
                    })
                }
            }
        ))
    }
}

impl Event {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        let variants = match input.data {
            syn::Data::Enum(data) => data.variants,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "Event can only be applied to enums",
                ))
            }
        };

        let aggregate: TokenStream = input
            .attrs
            .into_iter()
            .find_map(|attr| {
                let segment = attr.path.segments.first()?;
                if segment.ident != "aggregate" {
                    return None;
                }

                let mut tokens = attr.tokens.into_iter();
                if !matches!(tokens.next()?, TokenTree::Punct(punct) if punct.as_char() == '=') {
                    return None;
                }

                match tokens.next()? {
                    TokenTree::Literal(lit) => Some(lit.to_string().trim_matches('"').parse()),
                    _ => None,
                }
            })
            .ok_or_else(|| {
                syn::Error::new(ident.span(), "missing attribute #[aggregate = MyAggregate]")
            })??;

        Ok(Event {
            aggregate,
            ident,
            variants,
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_event = self.expand_impl_event()?;

        Ok(expanded_impl_event)
    }
}
