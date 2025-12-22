use std::{fs::OpenOptions, io::Write, path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use async_nats::Client;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::harness::TestEnv;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Step {
    InstallPack {
        pack_id: String,
    },
    StartService {
        name: String,
    },
    HttpPost {
        url: String,
        body: Value,
    },
    NatsPublish {
        subject: String,
        payload: Value,
    },
    AwaitNats {
        subject: String,
        expected: Option<Value>,
        timeout_ms: Option<u64>,
    },
    AssertJson {
        actual: Value,
        expected: Value,
    },
}

pub struct ScenarioRunner {
    nats_url: String,
    observations: PathBuf,
    subscribers: HashMap<String, async_nats::Subscriber>,
}

impl ScenarioRunner {
    pub fn new(env: &TestEnv) -> Result<Self> {
        let observations = env.artifacts_dir().join("observations.jsonl");
        Ok(Self {
            nats_url: env.nats_url(),
            observations,
            subscribers: HashMap::new(),
        })
    }

    pub async fn run(&mut self, scenario: &Scenario) -> Result<()> {
        let mut nats: Option<Client> = None;
        for step in &scenario.steps {
            match step {
                Step::NatsPublish { subject, payload } => {
                    let client = Self::ensure_nats(&mut nats, &self.nats_url).await?;
                    if !self.subscribers.contains_key(subject) {
                        let sub = client.subscribe(subject.clone()).await?;
                        self.subscribers.insert(subject.clone(), sub);
                    }
                    let bytes = serde_json::to_vec(payload)?;
                    client.publish(subject.clone(), bytes.into()).await?;
                    client.flush().await?;
                    self.record(
                        "nats_publish",
                        json!({"subject": subject, "payload": payload}),
                    )?;
                }
                Step::AwaitNats {
                    subject,
                    expected,
                    timeout_ms,
                } => {
                    let client = Self::ensure_nats(&mut nats, &self.nats_url).await?;
                    if !self.subscribers.contains_key(subject) {
                        let sub = client.subscribe(subject.clone()).await?;
                        self.subscribers.insert(subject.clone(), sub);
                    }
                    let sub = self
                        .subscribers
                        .get_mut(subject)
                        .ok_or_else(|| anyhow::anyhow!("missing subscriber for {}", subject))?;
                    let duration = Duration::from_millis(timeout_ms.unwrap_or(5_000));
                    let msg = tokio::time::timeout(duration, sub.next())
                        .await
                        .context("awaiting NATS message timed out")?
                        .ok_or_else(|| anyhow::anyhow!("subscription ended before message"))?;
                    let payload_val: Value =
                        serde_json::from_slice(&msg.payload).unwrap_or_else(|_| {
                            Value::String(String::from_utf8_lossy(&msg.payload).to_string())
                        });
                    if let Some(expected) = expected
                        && payload_val != *expected
                    {
                        bail!("awaited NATS payload did not match expected");
                    }
                    self.record(
                        "await_nats",
                        json!({"subject": subject, "payload": payload_val}),
                    )?;
                }
                Step::AssertJson { actual, expected } => {
                    if actual != expected {
                        bail!("assert json mismatch: actual {actual:?} expected {expected:?}");
                    }
                    self.record(
                        "assert_json",
                        json!({"actual": actual, "expected": expected}),
                    )?;
                }
                Step::InstallPack { pack_id } => {
                    self.record("install_pack_stub", json!({"pack_id": pack_id}))?;
                }
                Step::StartService { name } => {
                    self.record("start_service_stub", json!({"name": name}))?;
                }
                Step::HttpPost { url, body } => {
                    self.record("http_post_stub", json!({"url": url, "body": body}))?;
                }
            }
        }
        Ok(())
    }

    async fn ensure_nats(nats: &mut Option<Client>, url: &str) -> Result<Client> {
        if let Some(client) = nats.clone() {
            return Ok(client);
        }
        let client = async_nats::connect(url)
            .await
            .with_context(|| format!("failed to connect to NATS at {url}"))?;
        *nats = Some(client.clone());
        Ok(client)
    }

    fn record(&self, step: &str, data: Value) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.observations)
            .with_context(|| format!("failed to open {}", self.observations.display()))?;
        let line = json!({
            "step": step,
            "data": data,
        });
        writeln!(file, "{}", serde_json::to_string(&line)?)
            .with_context(|| format!("failed to write {}", self.observations.display()))?;
        Ok(())
    }
}
