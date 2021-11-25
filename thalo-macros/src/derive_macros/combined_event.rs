use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

pub struct CombinedEvent {
    event_types: Vec<syn::Type>,
    ident: syn::Ident,
}

impl CombinedEvent {
    fn expand_impl_combined_event(&self) -> TokenStream {
        let Self { event_types, ident } = self;

        quote!(
            impl ::thalo::CombinedEvent for #ident {
                fn aggregate_types() -> ::std::vec::Vec<&'static str> {
                    vec![
                        #( <#event_types as ::thalo::Event>::Aggregate::aggregate_type(), )*
                    ]
                }
            }
        )
    }
}

impl CombinedEvent {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        let variants = match input.data {
            syn::Data::Enum(data) => data.variants,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "CombinedEvent can only be applied to enums",
                ))
            }
        };

        let event_types = variants
            .into_iter()
            .map(|variant| match variant.fields {
                syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) if unnamed.len() == 1 => {
                    Ok(unnamed.into_iter().next().unwrap().ty)
                }
                _ => Err(syn::Error::new(
                    variant.fields.span(),
                    format!(
                        "variant must contain one unnamed field. eg `{}({0}Event)`",
                        variant.ident
                    ),
                )),
            })
            .collect::<Result<_, _>>()?;

        Ok(CombinedEvent { event_types, ident })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_combined_event = self.expand_impl_combined_event();

        Ok(expanded_impl_combined_event)
    }
}
