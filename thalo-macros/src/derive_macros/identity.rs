use proc_macro2::TokenStream;
use quote::quote;

pub struct Identity {
    ident: syn::Ident,
    identity_field: syn::Field,
}

impl Identity {
    fn expand_impl_identity(&self) -> syn::Result<TokenStream> {
        let Self {
            ident,
            identity_field,
        } = self;

        let field_ident = identity_field.ident.as_ref().unwrap();

        Ok(quote!(
            impl ::thalo::Identity for #ident {
                fn identity(&self) -> &str {
                    &self.#field_ident
                }

                fn new_with_id(id: String) -> Self {
                    #[allow(clippy::needless_update)]
                    Self {
                        #field_ident: id,
                        ..Default::default()
                    }
                }
            }
        ))
    }
}

impl Identity {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        let fields = match input.data {
            syn::Data::Struct(data) => data.fields,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "Identity can only be applied to structs",
                ))
            }
        };

        let identity_field = fields
            .into_iter()
            .find(|field| {
                field
                    .attrs
                    .iter()
                    .any(|attr| match attr.path.segments.first() {
                        Some(segment) => segment.ident == "identity",
                        None => false,
                    })
            })
            .ok_or_else(|| {
                syn::Error::new(
                    ident.span(),
                    "a field must be marked as the aggregate's identity with #[identity]",
                )
            })?;

        Ok(Identity {
            ident,
            identity_field,
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_aggregate_identity = self.expand_impl_identity()?;

        Ok(expanded_impl_aggregate_identity)
    }
}
