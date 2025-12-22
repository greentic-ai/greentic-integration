use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::{Map, Value};

pub struct Fixture;

impl Fixture {
    /// Load a JSON fixture relative to `fixtures/`.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Value> {
        let full = fixtures_root().join(path.as_ref());
        let data = fs::read_to_string(&full)
            .with_context(|| format!("failed to read fixture {}", full.display()))?;
        serde_json::from_str(&data)
            .with_context(|| format!("invalid JSON in fixture {}", full.display()))
    }

    /// Load a text fixture relative to `fixtures/`.
    pub fn load_text(path: impl AsRef<Path>) -> Result<String> {
        let full = fixtures_root().join(path.as_ref());
        fs::read_to_string(&full)
            .with_context(|| format!("failed to read fixture {}", full.display()))
    }
}

/// Normalize JSON by dropping unstable fields (timestamps, trace/span IDs, UUID-ish strings).
pub fn normalize_json(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(normalize_map(map)),
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json).collect()),
        Value::String(s) if is_uuid_like(&s) => Value::String("<redacted-uuid>".into()),
        other => other,
    }
}

fn normalize_map(map: Map<String, Value>) -> Map<String, Value> {
    let mut cleaned = Map::new();
    for (key, val) in map.into_iter() {
        if is_unstable_field(&key) {
            continue;
        }
        cleaned.insert(key.to_string(), normalize_json(val));
    }
    cleaned
}

fn is_unstable_field(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    matches!(
        k.as_str(),
        "timestamp"
            | "timestamp_ms"
            | "created_at"
            | "updated_at"
            | "trace_id"
            | "span_id"
            | "request_id"
            | "correlation_id"
            | "uuid"
    ) || k.ends_with("_id") && (k.contains("trace") || k.contains("span"))
}

fn is_uuid_like(s: &str) -> bool {
    let hex = |c: char| c.is_ascii_hexdigit();
    s.len() == 36
        && s.chars()
            .enumerate()
            .all(|(i, c)| matches!(i, 8 | 13 | 18 | 23) && c == '-' || hex(c))
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("fixtures"))
        .unwrap_or_else(|| PathBuf::from("fixtures"))
}
