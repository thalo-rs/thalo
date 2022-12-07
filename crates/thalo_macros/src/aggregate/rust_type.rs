use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use esdl::schema::{
    CommandEvents, CustomType, Event, EventOpt, Param, RepeatableType, Scalar, TypeOpt, TypeRef,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;

pub trait ToRustType {
    fn to_rust_type(&self) -> syn::Type;
}

impl ToRustType for CommandEvents {
    fn to_rust_type(&self) -> syn::Type {
        match self {
            CommandEvents::Single(event_opt) => event_opt.to_rust_type(),
            CommandEvents::Tuple(events) => {
                let types = events.iter().map(|event| event.to_rust_type());
                parse_quote!(( #( #types ),* ))
            }
        }
    }
}

impl ToRustType for Event {
    fn to_rust_type(&self) -> syn::Type {
        let name = format_ident!("{}", self.name);
        parse_quote!(#name)
    }
}

impl ToRustType for EventOpt {
    fn to_rust_type(&self) -> syn::Type {
        match self {
            EventOpt::Optional(event) => {
                let ty = event.to_rust_type();
                parse_quote!(std::option::Option<#ty>)
            }
            EventOpt::Required(event) => event.to_rust_type(),
        }
    }
}

impl ToRustType for RepeatableType {
    fn to_rust_type(&self) -> syn::Type {
        match self {
            RepeatableType::Single(type_opt) => type_opt.to_rust_type(),
            RepeatableType::OptionalArray(type_opt) => {
                let ty = type_opt.to_rust_type();
                parse_quote!(
                    std::option::Option<std::vec::Vec<#ty>>
                )
            }
            RepeatableType::RequiredArray(type_opt) => {
                let ty = type_opt.to_rust_type();
                parse_quote!(
                    std::vec::Vec<#ty>
                )
            }
        }
    }
}

impl ToRustType for TypeOpt {
    fn to_rust_type(&self) -> syn::Type {
        match self {
            TypeOpt::Optional(type_ref) => {
                let ty = type_ref.to_rust_type();
                parse_quote!(std::option::Option<#ty>)
            }
            TypeOpt::Required(type_ref) => type_ref.to_rust_type(),
        }
    }
}

impl ToRustType for TypeRef {
    fn to_rust_type(&self) -> syn::Type {
        match self {
            TypeRef::Scalar(scalar) => scalar.to_rust_type(),
            TypeRef::Custom(CustomType { name, .. }) => parse_quote!(#name),
        }
    }
}

impl ToRustType for Scalar {
    fn to_rust_type(&self) -> syn::Type {
        match self {
            Scalar::String => parse_quote!(String),
            Scalar::Int => parse_quote!(i32),
            Scalar::Long => parse_quote!(i64),
            Scalar::Float => parse_quote!(f32),
            Scalar::Double => parse_quote!(f64),
            Scalar::Bool => parse_quote!(bool),
            Scalar::Bytes => parse_quote!(std::vec::Vec<u8>),
        }
    }
}

pub trait ToEventsType {
    fn to_events_type(&self, ident: &syn::Ident, events_ident: &syn::Ident) -> TokenStream;
}

impl ToEventsType for CommandEvents {
    fn to_events_type(&self, ident: &syn::Ident, events_ident: &syn::Ident) -> TokenStream {
        match self {
            CommandEvents::Single(event_opt) => {
                let variant = format_ident!("{}", event_opt.event_name());

                match event_opt {
                    EventOpt::Optional(_) => {
                        quote! {
                            match #ident {
                                Some(event) => vec![#events_ident::#variant(event)],
                                None => vec![]
                            }
                        }
                    }
                    EventOpt::Required(_) => {
                        quote! {
                            vec![#events_ident::#variant(#ident)]
                        }
                    }
                }
            }
            CommandEvents::Tuple(events) => {
                let events_len = events.len();
                let pushes = events.iter().map(|event_opt| {
                    let variant = format_ident!("{}", event_opt.event_name());
                    match event_opt {
                        EventOpt::Optional(_) => {
                            quote! {
                                if let Some(event) = #ident {
                                    events.push(event);
                                }
                            }
                        }
                        EventOpt::Required(_) => {
                            quote!(events.push(#events_ident::#variant(#ident));)
                        }
                    }
                });

                quote! {{
                    let mut events = Vec::with_capacity(#events_len);
                    #( #pushes )*
                    events
                }}
            }
        }
    }
}

pub trait EventName {
    fn event_name(&self) -> &str;
}

impl EventName for EventOpt {
    fn event_name(&self) -> &str {
        match self {
            EventOpt::Optional(event) => &event.name,
            EventOpt::Required(event) => &event.name,
        }
    }
}

#[derive(Clone, Copy, Hash, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DeriveTrait {
    Clone,
    Copy,
    Hash,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
}

impl DeriveTrait {
    fn all() -> HashSet<Self> {
        HashSet::from_iter([
            DeriveTrait::Clone,
            DeriveTrait::Copy,
            DeriveTrait::Hash,
            DeriveTrait::Debug,
            DeriveTrait::Default,
            DeriveTrait::Eq,
            DeriveTrait::PartialEq,
            DeriveTrait::Ord,
            DeriveTrait::PartialOrd,
            DeriveTrait::Serialize,
            DeriveTrait::Deserialize,
        ])
    }
}

impl ToRustType for DeriveTrait {
    fn to_rust_type(&self) -> syn::Type {
        use DeriveTrait::*;

        match self {
            Clone => parse_quote!(Clone),
            Copy => parse_quote!(Copy),
            Hash => parse_quote!(Hash),
            Debug => parse_quote!(Debug),
            Default => parse_quote!(Default),
            Eq => parse_quote!(Eq),
            PartialEq => parse_quote!(PartialEq),
            Ord => parse_quote!(Ord),
            PartialOrd => parse_quote!(PartialOrd),
            Serialize => parse_quote!(serde::Serialize),
            Deserialize => parse_quote!(serde::Deserialize),
        }
    }
}

impl fmt::Display for DeriveTrait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DeriveTrait::*;

        match self {
            Clone => write!(f, "Clone"),
            Copy => write!(f, "Copy"),
            Hash => write!(f, "Hash"),
            Debug => write!(f, "Debug"),
            Default => write!(f, "Default"),
            Eq => write!(f, "Eq"),
            PartialEq => write!(f, "PartialEq"),
            Ord => write!(f, "Ord"),
            PartialOrd => write!(f, "PartialOrd"),
            Serialize => write!(f, "serde::Serialize"),
            Deserialize => write!(f, "serde::Deserialize"),
        }
    }
}

pub trait RustTypeDerives {
    fn derives(&self) -> HashSet<DeriveTrait>;
}

impl RustTypeDerives for Vec<Param> {
    fn derives(&self) -> HashSet<DeriveTrait> {
        self.iter()
            .fold(DeriveTrait::all(), |acc, param| &acc & &param.derives())
    }
}

impl RustTypeDerives for Param {
    fn derives(&self) -> HashSet<DeriveTrait> {
        self.ty.derives()
    }
}

impl RustTypeDerives for HashMap<String, RepeatableType> {
    fn derives(&self) -> HashSet<DeriveTrait> {
        self.iter()
            .fold(DeriveTrait::all(), |acc, (_, ty)| &acc & &ty.derives())
    }
}

impl RustTypeDerives for RepeatableType {
    fn derives(&self) -> HashSet<DeriveTrait> {
        match self {
            RepeatableType::Single(type_opt) => type_opt.derives(),
            RepeatableType::OptionalArray(type_opt) => type_opt.derives(),
            RepeatableType::RequiredArray(type_opt) => type_opt.derives(),
        }
    }
}

impl RustTypeDerives for TypeOpt {
    fn derives(&self) -> HashSet<DeriveTrait> {
        match self {
            TypeOpt::Optional(type_ref) => type_ref.derives(),
            TypeOpt::Required(type_ref) => type_ref.derives(),
        }
    }
}

impl RustTypeDerives for TypeRef {
    fn derives(&self) -> HashSet<DeriveTrait> {
        match self {
            TypeRef::Scalar(scalar) => scalar.derives(),
            TypeRef::Custom(custom_type) => custom_type.fields.derives(),
        }
    }
}

impl RustTypeDerives for Scalar {
    fn derives(&self) -> HashSet<DeriveTrait> {
        use DeriveTrait::*;

        match self {
            Scalar::String => HashSet::from_iter([
                Clone,
                Hash,
                Debug,
                Default,
                Eq,
                PartialEq,
                Ord,
                PartialOrd,
                Serialize,
                Deserialize,
            ]),
            Scalar::Int => DeriveTrait::all(),
            Scalar::Long => DeriveTrait::all(),
            Scalar::Float => HashSet::from_iter([
                Clone,
                Copy,
                Debug,
                Default,
                PartialEq,
                PartialOrd,
                Serialize,
                Deserialize,
            ]),
            Scalar::Double => HashSet::from_iter([
                Clone,
                Copy,
                Debug,
                Default,
                PartialEq,
                PartialOrd,
                Serialize,
                Deserialize,
            ]),
            Scalar::Bool => DeriveTrait::all(),
            Scalar::Bytes => HashSet::from_iter([
                Clone,
                Copy,
                Debug,
                Default,
                Eq,
                PartialEq,
                Ord,
                PartialOrd,
                Serialize,
                Deserialize,
            ]),
        }
    }
}
