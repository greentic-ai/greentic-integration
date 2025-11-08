use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_yaml_bw as serde_yaml;

#[derive(Debug, Deserialize)]
pub struct CapabilityDoc {
    pub reference_provider: String,
    pub simulator_provider: String,
    pub providers: BTreeMap<String, ProviderEntry>,
    pub downgrades: Vec<Downgrade>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderEntry {
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Downgrade {
    pub capability: String,
    pub reason: String,
}

impl CapabilityDoc {
    pub fn simulator_capabilities(&self) -> Option<BTreeSet<String>> {
        self.providers
            .get(&self.simulator_provider)
            .map(|entry| entry.capabilities.iter().cloned().collect())
    }

    pub fn reference_capabilities(&self) -> Option<BTreeSet<String>> {
        self.providers
            .get(&self.reference_provider)
            .map(|entry| entry.capabilities.iter().cloned().collect())
    }
}

pub fn load_capabilities(path: &Path) -> Result<CapabilityDoc, serde_yaml::Error> {
    let data = fs::read_to_string(path).expect("capabilities file missing");
    serde_yaml::from_str(&data)
}

pub fn capabilities_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("capabilities")
        .join("providers.yaml")
}
