use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use anyhow::{Context, Result};
use async_nats::Client;
use futures::StreamExt;
use greentic_integration::harness::TestEnv;
use serde_json::json;
use tokio::time::timeout;

#[tokio::test]
async fn e2e_multi_tenant_isolation() -> Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_multi_tenant_isolation: docker daemon not available");
        return Ok(());
    }

    unsafe {
        std::env::set_var("E2E_TEST_NAME", "e2e_multi_tenant_isolation");
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    // Tenant secrets scoped per tenant.
    env.write_tenant_secret("tenant-a", "API_TOKEN", "secret-A")?;
    env.write_tenant_secret("tenant-b", "API_TOKEN", "secret-B")?;

    let nats = async_nats::connect(env.nats_url()).await?;

    // Subscribe to both tenant subjects and record envelopes.
    let mut sub = nats.subscribe("tenant.*.request").await?;

    publish_request(&nats, "tenant-a", "hello").await?;
    publish_request(&nats, "tenant-b", "hello").await?;

    let mut seen: HashMap<String, serde_json::Value> = HashMap::new();
    for _ in 0..2 {
        let msg = timeout(Duration::from_secs(2), sub.next())
            .await
            .context("timeout waiting for tenant message")?
            .context("subscription ended unexpectedly")?;
        let subject = msg.subject.to_string();
        let payload: serde_json::Value =
            serde_json::from_slice(&msg.payload).unwrap_or_else(|_| json!({"raw": msg.payload}));
        seen.insert(subject, payload);
    }

    let a_subject = "tenant.tenant-a.request";
    let b_subject = "tenant.tenant-b.request";
    assert!(seen.contains_key(a_subject), "tenant-a message missing");
    assert!(seen.contains_key(b_subject), "tenant-b message missing");

    // Process messages with tenant-scoped secrets and persist per-tenant state.
    let response_a = process_for_tenant(&env, "tenant-a", seen.get(a_subject).unwrap())?;
    let response_b = process_for_tenant(&env, "tenant-b", seen.get(b_subject).unwrap())?;

    // Outputs differ due to tenant secrets.
    assert_ne!(response_a, response_b);
    assert!(response_a.contains("secret-A"));
    assert!(response_b.contains("secret-B"));

    // Verify state isolation.
    let state_a = read_state(&env, "tenant-a")?;
    let state_b = read_state(&env, "tenant-b")?;
    assert!(state_a.contains("tenant-a"));
    assert!(state_b.contains("tenant-b"));
    assert!(!state_a.contains("tenant-b"));
    assert!(!state_b.contains("tenant-a"));

    env.down().await?;
    Ok(())
}

async fn publish_request(nats: &Client, tenant: &str, msg: &str) -> Result<()> {
    let subject = format!("tenant.{tenant}.request");
    let payload = json!({
        "tenant": tenant,
        "message": msg,
    });
    nats.publish(subject, serde_json::to_vec(&payload)?.into())
        .await
        .context("failed to publish request")?;
    nats.flush().await?;
    Ok(())
}

fn process_for_tenant(env: &TestEnv, tenant: &str, payload: &serde_json::Value) -> Result<String> {
    let tenant_dir = env.tenant_artifacts_dir(tenant)?;
    let secrets_path = tenant_dir.join("secrets.json");
    let secrets: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&secrets_path)
            .with_context(|| format!("failed to read {}", secrets_path.display()))?,
    )
    .context("invalid tenant secrets json")?;

    let token = secrets
        .get("API_TOKEN")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let response = format!("tenant={tenant}, token={token}, msg={}", payload["message"]);

    let state_path = tenant_dir.join("state.json");
    fs::write(
        &state_path,
        json!({
            "tenant": tenant,
            "response": response,
            "origin": payload
        })
        .to_string(),
    )
    .with_context(|| format!("failed to write {}", state_path.display()))?;

    Ok(response)
}

fn read_state(env: &TestEnv, tenant: &str) -> Result<String> {
    let tenant_dir = env.tenant_artifacts_dir(tenant)?;
    let state_path = tenant_dir.join("state.json");
    fs::read_to_string(&state_path)
        .with_context(|| format!("failed to read {}", state_path.display()))
}
