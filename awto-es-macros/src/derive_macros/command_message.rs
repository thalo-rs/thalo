use proc_macro2::TokenStream;
use quote::quote;

pub struct CommandMessage {
    ident: syn::Ident,
}

impl CommandMessage {
    fn expand_impl_message(&self) -> syn::Result<TokenStream> {
        let Self { ident } = self;

        Ok(quote!(
            impl ::actix::Message for #ident {
                type Result = ::std::result::Result<::std::vec::Vec<::awto_es::AggregateEventOwned>, ::awto_es::Error>;
            }
        ))
    }
}

impl CommandMessage {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(CommandMessage { ident })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_command = self.expand_impl_message()?;

        Ok(expanded_impl_command)
    }
}
