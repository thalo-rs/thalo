use std::collections::{BTreeMap, HashMap};

use semver::{Version, VersionReq};

use crate::module::{ModuleID, ModuleName};

#[derive(Default)]
pub struct Registry {
    pub modules: HashMap<ModuleName, ModuleVersions>,
}

#[derive(Default)]
pub struct ModuleVersions(BTreeMap<Version, Vec<u8>>);

impl Registry {
    pub fn get_module(&self, name: &str, req: &VersionReq) -> Option<(&Version, &[u8])> {
        self.modules
            .get(name)
            .and_then(|versions| versions.get(req))
    }

    pub fn get_module_latest(&self, name: &str) -> Option<(&Version, &[u8])> {
        self.modules
            .get(name)
            .and_then(|versions| versions.get_latest())
    }

    pub fn get_module_versions(&self, name: &str) -> Option<&ModuleVersions> {
        self.modules.get(name)
    }

    pub fn add_module(&mut self, module_id: ModuleID, module: Vec<u8>) -> Option<Vec<u8>> {
        let module_versions = self.modules.entry(module_id.name).or_default();
        module_versions.insert(module_id.version, module)
    }
}

impl ModuleVersions {
    pub fn get(&self, req: &VersionReq) -> Option<(&Version, &[u8])> {
        self.0
            .iter()
            .rev()
            .find(|(version, _)| req.matches(version))
            .map(|(version, module)| (version, module.as_slice()))
    }

    pub fn get_latest(&self) -> Option<(&Version, &[u8])> {
        self.0
            .iter()
            .last()
            .map(|(version, module)| (version, module.as_slice()))
    }

    pub fn get_all(&self, req: Option<&VersionReq>) -> Vec<(&Version, &[u8])> {
        match req {
            Some(req) => self
                .0
                .iter()
                .filter(|(version, _)| req.matches(version))
                .map(|(version, module)| (version, module.as_slice()))
                .collect(),
            None => self
                .0
                .iter()
                .map(|(version, module)| (version, module.as_slice()))
                .collect(),
        }
    }

    pub fn insert(&mut self, version: Version, module: Vec<u8>) -> Option<Vec<u8>> {
        self.0.insert(version, module)
    }
}
