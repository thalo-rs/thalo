use std::str;

use better_bae::{FromAttributes, TryFromAttributes};
use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
};
use proc_macro2::TokenStream;
use quote::quote;

use crate::traits::DeriveMacro;

const UNKNOWN_RENAME_ALL_MESSAGE: &str = r#"Unknown rename type. Expected one of: "lowercase", "UPPERCASE", "PascalCase", "camelCase", "snake_case", "SCREAMING_SNAKE_CASE", "kebab-case", "SCREAMING-KEBAB-CASE""#;

pub(crate) struct EventType {
    ident: syn::Ident,
    variant_types: Vec<(syn::Ident, String)>,
}

#[derive(Default, FromAttributes)]
#[bae("thalo")]
struct Attrs {
    rename_all: Option<syn::LitStr>,
}

#[derive(Default, FromAttributes)]
#[bae("thalo")]
struct VariantAttrs {
    rename: Option<syn::LitStr>,
}

#[derive(Clone, Copy)]
#[allow(clippy::enum_variant_names)]
enum RenameAll {
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl str::FromStr for RenameAll {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use RenameAll::*;

        match s {
            "lowercase" => Ok(LowerCase),
            "UPPERCASE" => Ok(UpperCase),
            "PascalCase" => Ok(PascalCase),
            "camelCase" => Ok(CamelCase),
            "snake_case" => Ok(SnakeCase),
            "SCREAMING_SNAKE_CASE" => Ok(ScreamingSnakeCase),
            "kebab-case" => Ok(KebabCase),
            "SCREAMING-KEBAB-CASE" => Ok(ScreamingKebabCase),
            _ => Err(()),
        }
    }
}

impl DeriveMacro for EventType {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::try_from_attributes(&input.attrs)?.unwrap_or_default();
        let rename_all: Option<RenameAll> = attrs
            .rename_all
            .as_ref()
            .map(|rename_all| rename_all.value().parse())
            .transpose()
            .map_err(|_| {
                syn::Error::new(attrs.rename_all.unwrap().span(), UNKNOWN_RENAME_ALL_MESSAGE)
            })?;
        let ident = input.ident;

        let variants = match input.data {
            syn::Data::Enum(data) => data.variants,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "EventType can only be applied to enums",
                ))
            }
        };

        let variant_types: Vec<_> = variants
            .into_iter()
            .map(|variant| {
                let event_type = if let Some(rename) =
                    VariantAttrs::try_from_attributes(&variant.attrs)?
                        .unwrap_or_default()
                        .rename
                {
                    rename.value()
                } else if let Some(rename_all) = &rename_all {
                    match rename_all {
                        RenameAll::LowerCase => variant.ident.to_string().to_lowercase(),
                        RenameAll::UpperCase => variant.ident.to_string().to_uppercase(),
                        RenameAll::PascalCase => variant.ident.to_string().to_pascal_case(),
                        RenameAll::CamelCase => variant.ident.to_string().to_lower_camel_case(),
                        RenameAll::SnakeCase => variant.ident.to_string().to_snake_case(),
                        RenameAll::ScreamingSnakeCase => {
                            variant.ident.to_string().to_shouty_snake_case()
                        }
                        RenameAll::KebabCase => variant.ident.to_string().to_kebab_case(),
                        RenameAll::ScreamingKebabCase => {
                            variant.ident.to_string().to_shouty_kebab_case()
                        }
                    }
                } else {
                    variant.ident.to_string()
                };

                Result::<_, syn::Error>::Ok((variant.ident, event_type))
            })
            .collect::<Result<_, _>>()?;

        Ok(EventType {
            ident,
            variant_types,
        })
    }

    fn expand(self) -> syn::Result<proc_macro2::TokenStream> {
        let expanded_impl_event_type = self.expand_impl_event_type();

        Ok(expanded_impl_event_type)
    }
}

impl EventType {
    fn expand_impl_event_type(&self) -> TokenStream {
        let Self {
            ident,
            variant_types,
        } = self;

        let expanded_variants = variant_types.iter().map(
            |(variant_ident, event_type)| quote!(#ident::#variant_ident { .. } => #event_type),
        );

        quote! {
            #[automatically_derived]
            impl thalo::event::EventType for #ident {
                fn event_type(&self) -> &'static str {
                    match self {
                        #( #expanded_variants ),*
                    }
                }
            }
        }
    }
}
