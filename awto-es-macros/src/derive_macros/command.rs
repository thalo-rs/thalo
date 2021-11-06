use heck::ShoutySnekCase;
use proc_macro2::TokenStream;
use quote::quote;

pub struct Command {
    ident: syn::Ident,
    variants: syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
}

impl Command {
    fn expand_impl_command(&self) -> syn::Result<TokenStream> {
        let Self { ident, variants } = self;

        let variant_idents = variants.iter().map(|variant| &variant.ident);
        let variant_strings = variants
            .iter()
            .map(|variant| variant.ident.to_string().TO_SHOUTY_SNEK_CASE());

        Ok(quote!(
            impl ::awto_es::Command for #ident {
                fn command_type(&self) -> &'static str {
                    match self {
                        #( Self::#variant_idents { .. } => #variant_strings, )*
                    }
                }
            }
        ))
    }
}

impl Command {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;

        let variants = match input.data {
            syn::Data::Enum(data) => data.variants,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "Command can only be applied to enums",
                ))
            }
        };

        Ok(Command { ident, variants })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_impl_command = self.expand_impl_command()?;

        Ok(expanded_impl_command)
    }
}
