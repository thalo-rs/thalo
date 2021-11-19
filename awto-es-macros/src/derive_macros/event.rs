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

                fn aggregate_event<'a>(&'a self, aggregate_id: &'a str) -> ::awto_es::AggregateEvent<'a, #aggregate> {
                    ::awto_es::AggregateEvent::new(aggregate_id, self)
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

        let aggregate = input
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
