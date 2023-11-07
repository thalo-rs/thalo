use std::path::Path;
use std::str;

use anyhow::{anyhow, Result};
use itertools::Itertools;
use semver::{Version, VersionReq};
use sled::{Db, IVec, Tree};

#[derive(Clone)]
pub struct Registry {
    tree: Tree,
}

#[derive(Clone)]
pub struct ModuleVersions {
    tree: Tree,
    prefix: String,
}

impl Registry {
    pub fn new(db: &Db) -> Result<Self> {
        let tree = db.open_tree("modules")?;
        Ok(Registry { tree })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Registry::new(&db)
    }

    pub fn iter_all(&self) -> impl Iterator<Item = Result<(String, Version, IVec)>> {
        self.tree.iter().map(|res| {
            res.map_err(anyhow::Error::from).and_then(|(k, module)| {
                let id_str = std::str::from_utf8(&k)?;
                let (name_str, version_str) = id_str
                    .split_once('@')
                    .ok_or_else(|| anyhow!("missing @ separator"))?;
                let name = name_str.to_string();
                let version = Version::parse(version_str)?;
                Ok((name, version, module))
            })
        })
    }

    pub fn iter_all_latest(&self) -> impl Iterator<Item = Result<(String, Version, IVec)>> {
        self.tree
            .iter()
            .map(|res| {
                res.map_err(anyhow::Error::from).and_then(|(k, module)| {
                    let id_str = std::str::from_utf8(&k)?;
                    let (name_str, version_str) = id_str
                        .split_once('@')
                        .ok_or_else(|| anyhow!("missing @ separator"))?;
                    let name = name_str.to_string();
                    let version = Version::parse(version_str)?;
                    Ok((name, version, module))
                })
            })
            .coalesce(|res_a, res_b| match (res_a, res_b) {
                (Ok((name_a, version_a, module_a)), Ok((name_b, version_b, module_b))) => {
                    if name_a == name_b && version_b > version_a {
                        Ok(Ok((name_b, version_b, module_b)))
                    } else {
                        Err((
                            Ok((name_a, version_a, module_a)),
                            Ok((name_b, version_b, module_b)),
                        ))
                    }
                }
                (a, b) => Err((a, b)),
            })
    }

    pub fn module(&self, name: &str) -> Result<ModuleVersions> {
        if !name.chars().all(|c| c.is_ascii_alphabetic() || c == '_') {
            return Err(anyhow!(
                "module names must contain only alphabetic and `_` characters: '{name}'",
            ));
        }

        let prefix = format!("{name}@");
        Ok(ModuleVersions {
            tree: self.tree.clone(),
            prefix,
        })
    }

    pub fn load_module_latest_version(&self, name: impl Into<String>) -> Result<Option<Version>> {
        let mut prefix: String = name.into();
        prefix.push('@');

        let max_version = self
            .tree
            .scan_prefix(prefix.as_bytes())
            .keys()
            .map(|res| {
                res.map_err(anyhow::Error::from).and_then(|k| {
                    let version_str = std::str::from_utf8(&k[prefix.len()..])?;
                    Ok(Version::parse(version_str)?)
                })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max();

        Ok(max_version)
    }

    // pub async fn save_module(&self, name: &str, version: &Version, module: &[u8])
    // -> Result<()> {     let key = format!("{name}@{version}");
    //     self.tree.insert(key.as_bytes(), module)?;

    //     Ok(())
    // }

    // pub async fn load_all_schema_modules(&self) -> Result<Vec<RegistryRow>> {
    //     sqlx::query("SELECT name, version::text, schema, module FROM
    // registry.modules")         .fetch_all(&self.pool)
    //         .await?
    //         .into_iter()
    //         .map(|record| {
    //             Ok(RegistryRow {
    //                 name: record.get(0),
    //                 version: record
    //                     .get::<Option<String>, _>(1)
    //                     .ok_or_else(|| anyhow!("empty version in registry"))?
    //                     .parse()
    //                     .context("failed to parse module version")?,
    //                 schema: serde_json::from_value(record.get(2))
    //                     .context("failed to parse module schema")?,
    //                 module: record.get(3),
    //             })
    //         })
    //         .collect()
    // }
}

impl ModuleVersions {
    pub fn module_name(&self) -> &str {
        &self.prefix[..self.prefix.len() - 1]
    }

    pub fn get(&self, req: &VersionReq) -> Result<Option<(Version, IVec)>> {
        self.scan()
            .find(|res| {
                res.as_ref()
                    .map(|(version, _)| req.matches(version))
                    .unwrap_or(true)
            })
            .transpose()
    }

    pub fn get_latest(&self) -> Result<Option<(Version, IVec)>> {
        Ok(self
            .scan()
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .max_by(|(version_a, _), (version_b, _)| version_a.cmp(version_b)))
    }

    // pub fn get_all(&self, req: Option<&VersionReq>) -> Vec<(&Version, &[u8])> {
    //     match req {
    //         Some(req) => self
    //             .0
    //             .iter()
    //             .filter(|(version, _)| req.matches(version))
    //             .map(|(version, module)| (version, module.as_slice()))
    //             .collect(),
    //         None => self
    //             .0
    //             .iter()
    //             .map(|(version, module)| (version, module.as_slice()))
    //             .collect(),
    //     }
    // }

    pub async fn insert(&self, version: Version, module: impl Into<IVec>) -> Result<()> {
        let key = format!("{}{version}", self.prefix);
        self.tree.insert(key.as_bytes(), module)?;
        self.tree.flush_async().await?;

        Ok(())
    }

    fn scan(&self) -> impl Iterator<Item = Result<(Version, IVec)>> + '_ {
        self.tree.scan_prefix(self.prefix.as_bytes()).map(|res| {
            res.map_err(anyhow::Error::from).and_then(|(k, v)| {
                let version_str = std::str::from_utf8(&k[self.prefix.len()..])?;
                Ok((Version::parse(version_str)?, v))
            })
        })
    }
}
