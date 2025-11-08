use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("failed to read {path}: {source}")]
    Io {
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("failed to parse {path}: {source}")]
    Parse {
        source: serde_json::Error,
        path: PathBuf,
    },
    #[error("validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderReport {
    pub pack_id: String,
    pub scenario_id: String,
    pub message_count: usize,
    pub bot_messages: usize,
    pub user_messages: usize,
    pub system_messages: usize,
    pub transcript_hash: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PackManifest {
    id: String,
    name: String,
    description: String,
    #[allow(dead_code)]
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    r#type: Option<String>,
    scenarios: Vec<ScenarioEntry>,
}

#[derive(Debug, Deserialize)]
struct ScenarioEntry {
    id: String,
    entry: PathBuf,
    golden: PathBuf,
}

#[derive(Debug, Deserialize)]
struct GoldenSnapshot {
    scenario_id: String,
    transcript: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ScenarioSource {
    scenario: String,
    #[serde(default)]
    steps: Vec<Value>,
}

pub fn simulate_render(manifest_path: &Path) -> Result<Vec<RenderReport>, RenderError> {
    let manifest: PackManifest = read_json(manifest_path)?;
    if manifest.scenarios.is_empty() {
        return Err(RenderError::Validation(format!(
            "manifest {} must define at least one scenario",
            manifest_path.display()
        )));
    }

    let base_dir = manifest_path.parent().ok_or_else(|| {
        RenderError::Validation(format!(
            "manifest {} missing parent directory",
            manifest_path.display()
        ))
    })?;

    let mut reports = Vec::with_capacity(manifest.scenarios.len());
    for scenario in &manifest.scenarios {
        let entry_path = base_dir.join(&scenario.entry);
        let scenario_source: ScenarioSource = read_json(&entry_path)?;
        if scenario_source.scenario != scenario.id {
            return Err(RenderError::Validation(format!(
                "{} scenario mismatch (expected {}, found {})",
                entry_path.display(),
                scenario.id,
                scenario_source.scenario
            )));
        }
        if scenario_source.steps.is_empty() {
            return Err(RenderError::Validation(format!(
                "{} missing steps array",
                entry_path.display()
            )));
        }

        let golden_path = base_dir.join(&scenario.golden);
        let snapshot: GoldenSnapshot = read_json(&golden_path)?;
        if snapshot.scenario_id != scenario.id {
            return Err(RenderError::Validation(format!(
                "{} scenario_id mismatch (expected {}, found {})",
                golden_path.display(),
                scenario.id,
                snapshot.scenario_id
            )));
        }

        let (bot, user, system) = classify_messages(&snapshot.transcript);
        let hash = hash_transcript(&snapshot.transcript);
        reports.push(RenderReport {
            pack_id: manifest.id.clone(),
            scenario_id: scenario.id.clone(),
            message_count: snapshot.transcript.len(),
            bot_messages: bot,
            user_messages: user,
            system_messages: system,
            transcript_hash: hash,
        });
    }

    Ok(reports)
}

fn classify_messages(transcript: &[String]) -> (usize, usize, usize) {
    let mut bot = 0;
    let mut user = 0;
    let mut system = 0;
    for line in transcript {
        let prefix = line.split_once(':').map(|(p, _)| p).unwrap_or("");
        match prefix {
            "BOT" => bot += 1,
            "USER" => user += 1,
            "SYSTEM" => system += 1,
            _ => (),
        }
    }
    (bot, user, system)
}

fn hash_transcript(transcript: &[String]) -> String {
    let mut hasher = Sha256::new();
    for line in transcript {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, RenderError> {
    let data = fs::read_to_string(path).map_err(|source| RenderError::Io {
        source,
        path: path.to_path_buf(),
    })?;
    serde_json::from_str(&data).map_err(|source| RenderError::Parse {
        source,
        path: path.to_path_buf(),
    })
}
