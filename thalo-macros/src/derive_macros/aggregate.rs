use proc_macro2::TokenStream;

use crate::derive_macros::{AggregateChannel, AggregateType, Identity};

pub struct Aggregate {
    aggregate_channel: AggregateChannel,
    aggregate_type: AggregateType,
    identity: Identity,
}

impl Aggregate {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let aggregate_channel = AggregateChannel::new(input.clone())?;
        let aggregate_type = AggregateType::new(input.clone())?;
        let identity = Identity::new(input)?;

        Ok(Aggregate {
            aggregate_channel,
            aggregate_type,
            identity,
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_aggregate_channel = self.aggregate_channel.expand()?;
        let expanded_aggregate_type = self.aggregate_type.expand()?;
        let expanded_identity = self.identity.expand()?;

        Ok(TokenStream::from_iter([
            expanded_aggregate_channel,
            expanded_aggregate_type,
            expanded_identity,
        ]))
    }
}
