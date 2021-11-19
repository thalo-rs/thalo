use std::{fmt::Write, iter::FromIterator};

use heck::SnakeCase;
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};

pub struct PgRepository {
    columns: Vec<String>,
    fields: syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
    ident: syn::Ident,
    primary_key: String,
    repository_ident: syn::Ident,
    table_name: String,
}

impl PgRepository {
    fn expand_view_struct(&self) -> TokenStream {
        let Self {
            repository_ident, ..
        } = self;

        quote!(
            #[derive(Clone)]
            pub struct #repository_ident {
                pool: ::bb8_postgres::bb8::Pool<
                    ::bb8_postgres::PostgresConnectionManager<::bb8_postgres::tokio_postgres::NoTls>,
                >,
            }
        )
    }

    fn expand_impl_pg_repository(&self) -> TokenStream {
        let Self {
            ident,
            repository_ident,
            ..
        } = self;

        let expanded_save = self.expand_save();
        let expanded_load = self.expand_load();
        let expanded_last_event_id = self.expand_last_event_id();

        quote!(
            #[::async_trait::async_trait]
            impl ::awto_es::postgres::PgRepository for #repository_ident {
                type View = #ident;

                fn new(
                    pool: ::bb8_postgres::bb8::Pool<
                        ::bb8_postgres::PostgresConnectionManager<::bb8_postgres::tokio_postgres::NoTls>,
                    >,
                ) -> Self {
                    Self { pool }
                }

                async fn connect(
                    conn: &str,
                    tls: ::bb8_postgres::tokio_postgres::NoTls,
                ) -> ::std::result::Result<Self, ::bb8_postgres::tokio_postgres::Error> {
                    let manager = ::bb8_postgres::PostgresConnectionManager::new_from_stringlike(conn, tls)?;
                    let pool = ::bb8_postgres::bb8::Pool::builder()
                        .build(manager)
                        .await
                        .unwrap();

                    Ok(Self::new(pool))
                }

                async fn save(
                    &self,
                    view: &Self::View,
                    event_id: i64,
                    event_sequence: i64,
                ) -> ::std::result::Result<(), ::awto_es::Error> {
                    use ::awto_es::InternalError;
                    let conn = self
                        .pool
                        .get()
                        .await
                        .internal_error("could not get connection from pool")?;

                    #expanded_save
                }

                async fn load(&self, id: &str) -> ::std::result::Result<::std::option::Option<Self::View>, ::awto_es::Error> {
                    use ::awto_es::InternalError;
                    let conn = self
                        .pool
                        .get()
                        .await
                        .internal_error("could not get connection from pool")?;

                    #expanded_load
                }

                async fn last_event_id(&self) -> ::std::result::Result<::std::option::Option<i64>, ::awto_es::Error> {
                    use ::awto_es::InternalError;
                    let conn = self
                        .pool
                        .get()
                        .await
                        .internal_error("could not get connection from pool")?;

                    #expanded_last_event_id
                }
            }
        )
    }

    fn expand_save(&self) -> TokenStream {
        let Self {
            columns,
            fields,
            ident,
            primary_key,
            table_name,
            ..
        } = self;

        let mut q = String::new();
        write!(
            q,
            r#"INSERT INTO "{}" ("last_event_id", "last_event_sequence""#,
            table_name
        )
        .unwrap();
        for column in columns {
            write!(q, r#", "{}""#, column).unwrap();
        }
        write!(q, ") ").unwrap();
        write!(q, "VALUES ($1, $2").unwrap();
        for i in 0..columns.len() {
            write!(q, ", ${}", i + 3).unwrap();
        }
        write!(q, ") ").unwrap();
        write!(
            q,
            r#"ON CONFLICT ("{}") DO UPDATE SET "last_event_id" = $1, "last_event_sequence" = $2"#,
            primary_key
        )
        .unwrap();
        for (i, column) in columns.iter().skip(1).enumerate() {
            write!(q, r#", "{}" = ${}"#, column, i + 4).unwrap();
        }

        let field_idents = fields.iter().map(|field| field.ident.as_ref().unwrap());

        let error_string = format!("could not insert/update {} view", ident.to_string());

        quote!(
            conn.execute(#q,
                &[
                    &event_id,
                    &event_sequence,
                    #( &view.#field_idents, )*
                ],
            )
            .await
            .internal_error(#error_string)?;

            Ok(())
        )
    }

    fn expand_load(&self) -> TokenStream {
        let Self {
            columns,
            fields,
            ident,
            primary_key,
            table_name,
            ..
        } = self;

        let mut q = String::new();
        write!(q, r#"SELECT "#).unwrap();
        for (i, column) in columns.iter().skip(1).enumerate() {
            if i > 0 {
                write!(q, ", ").unwrap();
            }
            write!(q, r#""{}""#, column).unwrap();
        }
        write!(q, r#" FROM "{}" WHERE "{}" = $1"#, table_name, primary_key).unwrap();

        let error_string = format!("could not load {} view", ident.to_string());

        let mut fields_iter = fields.iter();
        let primary_key_ident = fields_iter.next().unwrap().ident.as_ref().unwrap();
        let (field_indexes, field_idents): (Vec<_>, Vec<_>) = fields
            .iter()
            .skip(1)
            .map(|field| field.ident.as_ref().unwrap())
            .enumerate()
            .unzip();

        quote!(
            let row = conn
                .query_opt(#q, &[&id])
                .await
                .internal_error(#error_string)?;

            Ok(row.map(|row| Self::View {
                #primary_key_ident: id.to_string(),
                #( #field_idents: row.get(#field_indexes), )*
            }))
        )
    }

    fn expand_last_event_id(&self) -> TokenStream {
        let Self { table_name, .. } = self;

        let q = format!(r#"SELECT MAX("last_event_id") FROM "{}""#, table_name);

        quote!(
            let row = conn
                .query_one(#q, &[])
                .await
                .internal_error("could not query max last_event_id")?;

            Ok(row.get(0))
        )
    }
}

impl PgRepository {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let ident = input.ident;
        let repository_ident = format_ident!("{}Repository", ident);

        let fields = match input.data {
            syn::Data::Struct(data) => match data.fields {
                syn::Fields::Named(fields) => fields.named,
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "PgView can only be applied to structs with named fields",
                    ))
                }
            },
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "PgView can only be applied to structs",
                ))
            }
        };

        let table_name = input
            .attrs
            .into_iter()
            .find_map(|attr| {
                let segment = attr.path.segments.first()?;
                if segment.ident != "table_name" {
                    return None;
                }

                let mut tokens = attr.tokens.into_iter();
                if !matches!(tokens.next()?, TokenTree::Punct(punct) if punct.as_char() == '=') {
                    return None;
                }

                match tokens.next()? {
                    TokenTree::Literal(lit) => Some(lit.to_string()),
                    _ => None,
                }
            })
            .unwrap_or_else(|| {
                let mut table_name_string = ident.to_string().to_snake_case();
                if table_name_string.ends_with("_view") {
                    table_name_string.truncate(table_name_string.len() - "_view".len());
                }
                table_name_string
            });

        let columns: Vec<_> = fields
            .iter()
            .map(|field| field.ident.as_ref().unwrap().to_string())
            .collect();

        let primary_key = columns
            .get(0)
            .ok_or_else(|| {
                syn::Error::new(
                    ident.span(),
                    "struct must have at least one field for primary key",
                )
            })?
            .clone();

        Ok(PgRepository {
            columns,
            ident,
            fields,
            primary_key,
            repository_ident,
            table_name,
        })
    }

    pub fn expand(self) -> syn::Result<TokenStream> {
        let expanded_view_struct = self.expand_view_struct();
        let expanded_impl_pg_repository = self.expand_impl_pg_repository();

        Ok(TokenStream::from_iter([
            expanded_view_struct,
            expanded_impl_pg_repository,
        ]))
    }
}
