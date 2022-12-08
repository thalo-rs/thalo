use anyhow::{anyhow, Context, Result};
use esdl::schema::Schema;
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Clone)]
pub struct Registry {
    pool: PgPool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct RegistryRow {
    pub name: String,
    pub version: Version,
    pub schema: Schema,
    pub module: Vec<u8>,
}

impl Registry {
    pub async fn connect(url: &str) -> Result<Self> {
        Ok(Registry {
            pool: PgPool::connect(url).await?,
        })
    }

    pub async fn load_module_latest_version(&self, module: &str) -> Result<Option<Version>> {
        let version = sqlx::query_scalar!(
            "SELECT MAX(version)::text as version FROM registry.modules WHERE name = $1",
            module
        )
        .fetch_one(&self.pool)
        .await?
        .and_then(|version| version.parse().ok());

        Ok(version)
    }

    pub async fn save_schema_module(&self, schema: &Schema, module: &[u8]) -> Result<()> {
        let schema_json = serde_json::to_value(schema)?;
        sqlx::query!(
            "INSERT INTO registry.modules (name, version, schema, module) VALUES ($1, $2::text::semver, $3, $4)",
            &schema.aggregate.name,
            &schema.version.to_string(),
            &schema_json,
            module,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn load_all_schema_modules(&self) -> Result<Vec<RegistryRow>> {
        sqlx::query!("SELECT name, version::text, schema, module FROM registry.modules")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|record| {
                Ok(RegistryRow {
                    name: record.name,
                    version: record
                        .version
                        .ok_or_else(|| anyhow!("empty version in registry"))?
                        .parse()
                        .context("failed to parse module version")?,
                    schema: serde_json::from_value(record.schema)
                        .context("failed to parse module schema")?,
                    module: record.module,
                })
            })
            .collect()
    }
}
