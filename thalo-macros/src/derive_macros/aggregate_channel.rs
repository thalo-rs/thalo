use proc_macro2::TokenStream;
use quote::quote;

pub struct AggregateChannel {
    ident: syn::Ident,
}

impl AggregateChannel {
    fn expand_impl_aggregate_channel(&self) -> syn::Result<TokenStream> {
        let Self { ident } = self;

        Ok(quote!(
            impl<Act, ES> ::thalo::SharedGlobal<Act, ES> for #ident
            where
                Act: ::thalo::AggregateActor<ES, Self>,
                ES: ::thalo::EventStore + Clone + Send + Sync + Unpin + 'static,
                <Self as ::thalo::Aggregate>::Command: ::thalo::Message<
                    Result = Result<Vec<::thalo::EventEnvelope<<Self as ::thalo::Aggregate>::Event>>, ::thalo::Error>,
                >,
            {
                type Value = ::thalo::AggregateActorPool<Act, ES, Self>;
            }

            #[::thalo::async_trait]
            impl<Act, ES> ::thalo::AggregateChannel<Act, ES> for #ident
            where
                Self: ::thalo::SharedGlobal<Act, ES, Value = ::thalo::AggregateActorPool<Act, ES, Self>>,
                Act: ::thalo::AggregateActor<ES, Self>,
                ES: ::thalo::EventStore + Clone + Send + Sync + Unpin + 'static,
                <Self as ::thalo::Aggregate>::Command: ::thalo::Message<
                    Result = Result<Vec<::thalo::EventEnvelope<<Self as ::thalo::Aggregate>::Event>>, ::thalo::Error>,
                >,
            {
                async fn do_send(
                    id: &str,
                    command: <Self as ::thalo::Aggregate>::Command,
                ) -> Result<(), ::thalo::Error> {
                    let value = <Self as ::thalo::SharedGlobal<Act, ES>>::get()
                        .ok_or_else(|| ::thalo::Error::AggregateChannelNotInitialised(<Self as ::thalo::AggregateType>::aggregate_type()))?;
                    let value = value.value();

                    <<Self as ::thalo::SharedGlobal<Act, ES>>::Value>::do_send(value, id, command).await
                }

                async fn send(
                    id: &str,
                    command: <Self as ::thalo::Aggregate>::Command,
                ) -> Result<Vec<::thalo::EventEnvelope<<Self as ::thalo::Aggregate>::Event>>, ::thalo::Error> {
                    let value = <Self as ::thalo::SharedGlobal<Act, ES>>::get()
                        .ok_or_else(|| ::thalo::Error::AggregateChannelNotInitialised(<Self as ::thalo::AggregateType>::aggregate_type()))?;
                    let value = value.value();

                    <<Self as ::thalo::SharedGlobal<Act, ES>>::Value>::send(value, id, command).await
                }
            }
        ))
    }
}

impl AggregateChannel {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        Ok(AggregateChannel { ident })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_aggregate_channel = self.expand_impl_aggregate_channel()?;

        Ok(expanded_impl_aggregate_channel)
    }
}
