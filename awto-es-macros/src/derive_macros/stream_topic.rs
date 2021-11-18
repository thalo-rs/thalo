use heck::SnakeCase;
use proc_macro2::TokenStream;
use quote::quote;

pub struct StreamTopic {
    ident: syn::Ident,
}

impl StreamTopic {
    fn expand_impl_stream_topic(&self) -> syn::Result<TokenStream> {
        let Self { ident } = self;

        let stream_topic_string = ident.to_string().to_snake_case();

        Ok(quote!(
            impl ::awto_es::StreamTopic for #ident {
                fn stream_topic() -> &'static str {
                    #stream_topic_string
                }
            }
        ))
    }
}

impl StreamTopic {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(StreamTopic { ident })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_stream_topic = self.expand_impl_stream_topic()?;

        Ok(expanded_impl_stream_topic)
    }
}
