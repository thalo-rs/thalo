use proc_macro2::TokenStream;

pub(crate) trait DeriveMacro: Sized {
    fn new(input: syn::DeriveInput) -> syn::Result<Self>;

    fn expand(self) -> syn::Result<TokenStream>;
}
