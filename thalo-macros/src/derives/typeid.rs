use better_bae::{FromAttributes, TryFromAttributes};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;

use crate::traits::DeriveMacro;

pub(crate) struct TypeId {
    attrs: Option<Attrs>,
    ident: syn::Ident,
}

#[derive(FromAttributes)]
#[bae("thalo")]
struct Attrs {
    type_id: syn::LitStr,
}

impl DeriveMacro for TypeId {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::try_from_attributes(&input.attrs)?;
        let ident = input.ident;

        Ok(TypeId { attrs, ident })
    }

    fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let expanded_impl_type_id = self.expand_impl_type_id();

        Ok(expanded_impl_type_id)
    }
}

impl TypeId {
    fn expand_impl_type_id(&self) -> TokenStream {
        let Self { attrs, ident } = self;

        let type_id_string = attrs
            .as_ref()
            .map(|attrs| attrs.type_id.value())
            .unwrap_or_else(|| ident.to_string().to_snake_case());

        quote! {
            #[automatically_derived]
            impl thalo::aggregate::TypeId for #ident {
                fn type_id() -> &'static str {
                    #type_id_string
                }
            }
        }
    }
}
