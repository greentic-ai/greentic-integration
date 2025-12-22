use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigLayers {
    pub defaults: Value,
    pub user: Option<Value>,
    pub project: Option<Value>,
    pub env: Option<Value>,
    pub cli: Option<Value>,
}

impl ConfigLayers {
    /// Merge layers with precedence: defaults < user < project < env < cli.
    pub fn merge(&self) -> Value {
        let mut merged = self.defaults.clone_or_null();
        if let Some(user) = &self.user {
            merged = merge_json(merged, user.clone());
        }
        if let Some(project) = &self.project {
            merged = merge_json(merged, project.clone());
        }
        if let Some(env) = &self.env {
            merged = merge_json(merged, env.clone());
        }
        if let Some(cli) = &self.cli {
            merged = merge_json(merged, cli.clone());
        }
        merged
    }
}

pub fn load_toml(path: &Path) -> Result<Value> {
    let data =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&data).with_context(|| format!("invalid TOML in {}", path.display()))?;
    let json = toml::to_string(&value).context("failed to serialize toml to string")?;
    serde_json::from_str(&json).context("failed to convert toml to json")
}

pub fn merge_json(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Object(mut a), Value::Object(b)) => {
            for (k, v) in b {
                let entry = a.remove(&k);
                let merged = match entry {
                    Some(existing) => merge_json(existing, v),
                    None => v,
                };
                a.insert(k, merged);
            }
            Value::Object(a)
        }
        (_, over) => over,
    }
}

#[derive(Debug, Serialize)]
pub struct SecretCheck {
    pub required: Vec<String>,
    pub provided: Vec<String>,
    pub missing: Vec<String>,
}

impl SecretCheck {
    pub fn new(required: Vec<String>) -> Self {
        Self {
            missing: required.clone(),
            required,
            provided: Vec::new(),
        }
    }
}

pub fn apply_secrets(
    required: &[String],
    secrets: &BTreeMap<String, String>,
) -> Result<SecretCheck> {
    let mut check = SecretCheck::new(required.to_vec());
    for key in required {
        if let Some(val) = secrets.get(key)
            && !val.trim().is_empty()
        {
            check.provided.push(key.clone());
        }
    }
    check.missing = required
        .iter()
        .filter(|k| !check.provided.contains(k))
        .cloned()
        .collect();
    if check.missing.is_empty() {
        Ok(check)
    } else {
        bail!(
            "missing secrets: {:?}. Remedy: set via CLI/env/config/secret store",
            check.missing
        );
    }
}

trait CloneOrNull {
    fn clone_or_null(&self) -> Value;
}

impl CloneOrNull for Value {
    fn clone_or_null(&self) -> Value {
        match self {
            Value::Null => Value::Null,
            other => other.clone(),
        }
    }
}
