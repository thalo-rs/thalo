use heck::SnakeCase;
use proc_macro2::TokenStream;
use quote::quote;

pub struct AggregateType {
    ident: syn::Ident,
}

impl AggregateType {
    fn expand_impl_aggregate_type(&self) -> syn::Result<TokenStream> {
        let Self { ident } = self;

        let aggregate_type_string = ident.to_string().to_snake_case();

        Ok(quote!(
            impl ::thalo::AggregateType for #ident {
                fn aggregate_type() -> &'static str {
                    #aggregate_type_string
                }
            }
        ))
    }
}

impl AggregateType {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(AggregateType { ident })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_aggregate_type = self.expand_impl_aggregate_type()?;

        Ok(expanded_impl_aggregate_type)
    }
}
