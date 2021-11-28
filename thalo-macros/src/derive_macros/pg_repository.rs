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
                pool: ::thalo::postgres::bb8::Pool<
                    ::thalo::postgres::PostgresConnectionManager<::thalo::postgres::tokio_postgres::NoTls>,
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
        let expanded_update_last_event = self.expand_update_last_event();
        let expanded_load_with_last_event_id = self.expand_load_with_last_event_id();
        let expanded_last_event_id = self.expand_last_event_id();
        let expanded_delete = self.expand_delete();

        quote!(
            #[::thalo::async_trait]
            impl ::thalo::postgres::PgRepository for #repository_ident {
                type View = #ident;

                fn new(
                    pool: ::thalo::postgres::bb8::Pool<
                        ::thalo::postgres::PostgresConnectionManager<::thalo::postgres::tokio_postgres::NoTls>,
                    >,
                ) -> Self {
                    Self { pool }
                }

                async fn connect(
                    conn: &str,
                    tls: ::thalo::postgres::tokio_postgres::NoTls,
                ) -> ::std::result::Result<Self, ::thalo::postgres::tokio_postgres::Error> {
                    let manager = ::thalo::postgres::PostgresConnectionManager::new_from_stringlike(conn, tls)?;
                    let pool = ::thalo::postgres::bb8::Pool::builder()
                        .build(manager)
                        .await
                        .unwrap();

                    Ok(Self::new(pool))
                }

                async fn save(
                    &self,
                    id: &str,
                    view: Option<&Self::View>,
                    event_id: i64,
                    event_sequence: i64,
                ) -> ::std::result::Result<(), ::thalo::Error> {
                    let mut conn = self
                        .pool
                        .get()
                        .await
                        .map_err(::thalo::Error::GetDbPoolConnectionError)?;

                    #expanded_save
                }

                async fn update_last_event(
                    &self,
                    id: &str,
                    event_id: i64,
                    event_sequence: i64,
                ) -> ::std::result::Result<(), ::thalo::Error> {
                    let conn = self
                        .pool
                        .get()
                        .await
                        .map_err(::thalo::Error::GetDbPoolConnectionError)?;

                    #expanded_update_last_event
                }

                async fn load_with_last_event_id(&self, id: &str) -> ::std::result::Result<::std::option::Option<(Self::View, i64)>, ::thalo::Error> {
                    let conn = self
                        .pool
                        .get()
                        .await
                        .map_err(::thalo::Error::GetDbPoolConnectionError)?;

                    #expanded_load_with_last_event_id
                }

                async fn last_event_id(&self) -> ::std::result::Result<::std::option::Option<i64>, ::thalo::Error> {
                    let conn = self
                        .pool
                        .get()
                        .await
                        .map_err(::thalo::Error::GetDbPoolConnectionError)?;

                    #expanded_last_event_id
                }

                async fn delete(&self, id: &str) -> ::std::result::Result<(), ::thalo::Error> {
                    let conn = self
                        .pool
                        .get()
                        .await
                        .map_err(::thalo::Error::GetDbPoolConnectionError)?;

                    #expanded_delete
                }
            }
        )
    }

    fn expand_save(&self) -> TokenStream {
        let Self {
            columns,
            fields,
            primary_key,
            table_name,
            ..
        } = self;

        let mut q = String::new();
        write!(q, r#"INSERT INTO "{}" "#, table_name).unwrap();
        write!(
            q,
            "({}) ",
            columns
                .iter()
                .map(|column| format!(r#""{}""#, column))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
        write!(q, "VALUES ").unwrap();
        write!(
            q,
            "({}) ",
            (0..columns.len())
                .map(|i| format!(r#"${}"#, i + 1))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
        write!(q, r#"ON CONFLICT ("{}") DO UPDATE SET "#, primary_key).unwrap();
        write!(
            q,
            "{}",
            columns
                .iter()
                .filter(|column| *column != primary_key)
                .enumerate()
                .map(|(i, column)| { format!(r#""{}" = ${}"#, column, i + 2) })
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();

        let field_idents = fields.iter().map(|field| field.ident.as_ref().unwrap());

        quote!(
            let t = conn
                .build_transaction()
                .isolation_level(::thalo::postgres::tokio_postgres::IsolationLevel::Serializable)
                .start()
                .await?;

            if let Some(view) = view {
                t.execute(
                    #q,
                    &[
                        #( &view.#field_idents, )*
                    ],
                )
                .await?;
            }

            t.execute(
                r#"
                    INSERT INTO "projection" ("id", "type", "event_id", "event_sequence")
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT ("id", "type") DO UPDATE SET
                    "event_id" = $3, "event_sequence" = $4
                "#,
                &[
                    &id,
                    &#table_name,
                    &event_id,
                    &event_sequence,
                ],
            )
            .await?;

            t.commit().await?;

            Ok(())
        )
    }

    fn expand_update_last_event(&self) -> TokenStream {
        let Self { table_name, .. } = self;

        let q = format!(
            r#"UPDATE "projection" SET "event_id" = $2, "event_sequence" = $3 WHERE "id" = $1 AND "type" = '{}'"#,
            table_name
        );

        quote!(
            conn.execute(
                #q,
                &[
                    &id,
                    &event_id,
                    &event_sequence,
                ],
            )
            .await?;

            Ok(())
        )
    }

    fn expand_load_with_last_event_id(&self) -> TokenStream {
        let Self {
            columns,
            fields,
            primary_key,
            table_name,
            ..
        } = self;

        let mut q = String::new();
        write!(q, r#"SELECT (SELECT "projection"."event_id" FROM "projection" WHERE "projection"."id" = $1 AND "projection"."type" = '{}')"#, table_name).unwrap();
        for column in columns.iter().skip(1) {
            write!(q, r#", "{}""#, column).unwrap();
        }
        write!(q, r#" FROM "{}" WHERE "{}" = $1"#, table_name, primary_key).unwrap();

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
                .await?;

            Ok(row.map(|row| {
                (
                    Self::View {
                        #primary_key_ident: id.to_string(),
                        #( #field_idents: row.get(#field_indexes + 1), )*
                    },
                    row.get(0)
                )
            }))
        )
    }

    fn expand_last_event_id(&self) -> TokenStream {
        let Self { table_name, .. } = self;

        let q = r#"SELECT MAX("event_id") FROM "projection" WHERE "type" = $1"#;

        quote!(
            let row = conn.query_one(#q, &[&#table_name]).await?;

            Ok(row.get(0))
        )
    }

    fn expand_delete(&self) -> TokenStream {
        let Self {
            primary_key,
            table_name,
            ..
        } = self;

        let q = format!(
            r#"DELETE FROM "{}" WHERE "{}" = $1"#,
            table_name, primary_key
        );

        quote!(
            conn.execute(#q, &[&id]).await?;

            Ok(())
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
