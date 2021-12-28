use better_bae::{FromAttributes, TryFromAttributes};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::traits::DeriveMacro;

pub(crate) struct Aggregate {
    attrs: Attrs,
    ident: syn::Ident,
    id_ident: syn::Ident,
    id_type: syn::Type,
}

#[derive(Default, FromAttributes)]
#[bae("thalo")]
struct Attrs {
    apply: Option<syn::LitStr>,
    event: Option<syn::LitStr>,
}

#[derive(Default, FromAttributes)]
#[bae("thalo")]
struct FieldAttrs {
    id: Option<()>,
}

impl DeriveMacro for Aggregate {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::try_from_attributes(&input.attrs)?.unwrap_or_default();
        let ident = input.ident;

        let fields = match input.data {
            syn::Data::Struct(data) => data.fields,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "Aggregate can only be applied to structs",
                ))
            }
        };

        let (id_ident, id_type) = fields
            .iter()
            .find_map(
                |field| match FieldAttrs::try_from_attributes(&field.attrs) {
                    Ok(Some(attrs)) if attrs.id.is_some() => Some(Result::<_, syn::Error>::Ok((
                        field.ident.as_ref().unwrap().clone(),
                        field.ty.clone(),
                    ))),
                    Ok(Some(_)) | Ok(None) => None,
                    Err(err) => Some(Err(err)),
                },
            )
            .unwrap_or_else(|| {
                fields
                    .iter()
                    .find_map(|field| {
                        if field
                            .ident
                            .as_ref()
                            .map(|ident| ident == "id")
                            .unwrap_or(false)
                        {
                            Some((field.ident.as_ref().unwrap().clone(), field.ty.clone()))
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| {
                        syn::Error::new(ident.span(), "a field must be marked with #[thalo(id)]")
                    })
            })?;

        Ok(Aggregate {
            attrs,
            ident,
            id_ident,
            id_type,
        })
    }

    fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_aggregate = self.expand_impl_aggregate();

        Ok(expanded_impl_aggregate)
    }
}

impl Aggregate {
    fn expand_impl_aggregate(&self) -> TokenStream {
        let Self {
            attrs,
            ident,
            id_ident,
            id_type,
        } = self;

        let event_ty = format_ident!(
            "{}",
            attrs
                .event
                .as_ref()
                .map(|event| event.value())
                .unwrap_or_else(|| format!("{}Event", ident.to_string()))
        );

        let apply_ident = attrs
            .apply
            .as_ref()
            .map(|apply| format_ident!("{}", apply.value()))
            .unwrap_or_else(|| format_ident!("apply"));

        quote! {
            #[automatically_derived]
            impl thalo::aggregate::Aggregate for #ident {
                type ID = #id_type;
                type Event = #event_ty;

                fn new(id: Self::ID) -> Self {
                    #ident {
                        id: #id_ident,
                        ..std::default::Default::default()
                    }
                }

                fn id(&self) -> &Self::ID {
                    &self.#id_ident
                }

                fn apply(&mut self, event: Self::Event) {
                    #apply_ident(self, event)
                }
            }
        }
    }
}
