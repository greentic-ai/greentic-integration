use std::fs;

use greentic_integration::harness::TestEnv;

#[tokio::test]
async fn e2e_smoke_creates_dirs_and_marker() -> anyhow::Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_smoke: docker daemon not available");
        return Ok(());
    }

    // Setting environment variables is unsafe in Rust 2024; guard it explicitly.
    unsafe {
        std::env::set_var("E2E_TEST_NAME", "e2e_smoke");
    }

    let env = TestEnv::up().await?;
    assert_eq!(env.name(), "e2e_smoke");

    env.healthcheck().await?;

    let logs = env.logs_dir();
    let artifacts = env.artifacts_dir();
    assert!(logs.is_dir(), "logs dir missing");
    assert!(artifacts.is_dir(), "artifacts dir missing");

    let ready = logs.join("READY");
    assert!(ready.is_file(), "READY marker missing");

    let snapshot = env.root().join("env.json");
    assert!(
        snapshot.is_file(),
        "env snapshot should exist at {}",
        snapshot.display()
    );

    // Ensure writes are possible.
    fs::write(artifacts.join("smoke.txt"), "ok").expect("write artifact");

    env.down().await?;
    Ok(())
}
