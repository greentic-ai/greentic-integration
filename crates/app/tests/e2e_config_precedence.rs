use std::collections::BTreeMap;
use std::path::PathBuf;

use greentic_integration::harness::{ConfigLayers, TestEnv, apply_secrets, load_toml};
use serde_json::json;

#[tokio::test]
async fn e2e_config_secrets_precedence() -> anyhow::Result<()> {
    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("fixtures")
        .join("config")
        .join("precedence");

    let defaults = json!({"secrets": {"API_TOKEN": ""}, "source": "defaults"});
    let user = load_toml(&fixtures_root.join("user.toml")).ok();
    let project = load_toml(&fixtures_root.join("project.toml")).ok();
    let env_layer = Some(json!({"secrets": {"API_TOKEN": "env-token"}, "source": "env"}));
    let cli_layer = Some(json!({"secrets": {"API_TOKEN": "cli-token"}, "source": "cli"}));

    let layers = ConfigLayers {
        defaults: defaults.clone(),
        user,
        project,
        env: env_layer,
        cli: cli_layer.clone(),
    };

    let merged = layers.merge();
    let secrets = merged
        .get("secrets")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect::<BTreeMap<_, _>>();

    // Phase 1: remove CLI layer to simulate missing secret.
    let missing_layers = ConfigLayers {
        defaults: defaults.clone(),
        user: None,
        project: None,
        env: None,
        cli: None,
    };
    let merged_missing = missing_layers.merge();
    let secrets_missing = merged_missing
        .get("secrets")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect::<BTreeMap<_, _>>();

    let required = vec!["API_TOKEN".to_string()];
    if let Ok(check_missing) = apply_secrets(&required, &secrets_missing) {
        panic!(
            "expected missing secret error, got provided={:?}, missing={:?}",
            check_missing.provided, check_missing.missing
        );
    }

    // Phase 2: with CLI layer present, succeeds.
    let check = apply_secrets(&required, &secrets)?;
    assert!(check.missing.is_empty());
    assert_eq!(check.provided, required);

    // Record artifacts.
    let artifacts = env.artifacts_dir().join("config");
    std::fs::create_dir_all(&artifacts)?;
    std::fs::write(
        artifacts.join("merged.json"),
        serde_json::to_vec_pretty(&merged)?,
    )?;
    std::fs::write(
        artifacts.join("missing.json"),
        serde_json::to_vec_pretty(&merged_missing)?,
    )?;
    std::fs::write(
        artifacts.join("secret_check.json"),
        serde_json::to_vec_pretty(&json!(check))?,
    )?;

    env.down().await?;
    Ok(())
}
