use crate::traits::DeriveMacro;

#[inline]
pub(crate) fn declare_derive_macro<M: DeriveMacro>(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match M::new(input) {
        Ok(derive) => derive
            .expand()
            .unwrap_or_else(syn::Error::into_compile_error)
            .into(),
        Err(err) => syn::Error::into_compile_error(err).into(),
    }
}
