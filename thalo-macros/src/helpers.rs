macro_rules! derive_macro {
    ($name: ident$(, $args: ident )* ) => {
        #[proc_macro_derive($name, attributes( $( $args ),* ))]
        #[allow(non_snake_case)]
        pub fn $name(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let input = syn::parse_macro_input!(input as syn::DeriveInput);

            match derive_macros::$name::new(input) {
                Ok(derive) => derive_macros::$name::expand(derive)
                    .unwrap_or_else(syn::Error::into_compile_error)
                    .into(),
                Err(err) => syn::Error::into_compile_error(err).into(),
            }
        }
    };
}

macro_rules! attribute_macro {
    ($name: ident, $name_camel: ident) => {
        #[proc_macro_attribute]
        pub fn $name(
            args: proc_macro::TokenStream,
            input: proc_macro::TokenStream,
        ) -> proc_macro::TokenStream {
            let args = syn::parse::Parser::parse(<syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>>::parse_terminated, args).unwrap();
            let input = syn::parse_macro_input!(input as syn::ItemImpl);

            match proc_macros::$name_camel::new(args, input) {
                Ok(proc) => proc_macros::$name_camel::expand(proc)
                    .unwrap_or_else(syn::Error::into_compile_error)
                    .into(),
                Err(err) => syn::Error::into_compile_error(err).into(),
            }
        }
    };
}

pub(crate) use attribute_macro;
pub(crate) use derive_macro;
