use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use greentic_integration::harness::TestEnv;
use serde_json::json;
use tokio::time::sleep;

#[tokio::test]
async fn e2e_retry_backoff_flaky_tool() -> Result<()> {
    if !greentic_integration::harness::docker_available() {
        eprintln!("skipping e2e_retry_backoff_flaky_tool: docker daemon not available");
        return Ok(());
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    let retry_dir = env.artifacts_dir().join("retry");
    if retry_dir.exists() {
        fs::remove_dir_all(&retry_dir)?;
    }
    fs::create_dir_all(&retry_dir)?;

    let attempts_log = retry_dir.join("attempts.jsonl");

    let max_retries = 3;
    let success_after = 3;
    let mut errors = Vec::new();
    let mut final_output = None;

    for attempt in 1..=max_retries {
        match flaky_tool(&attempts_log, success_after)? {
            Ok(val) => {
                final_output = Some(val);
                break;
            }
            Err(err) => {
                errors.push((attempt, err.to_string()));
                sleep(Duration::from_millis(50)).await;
            }
        }
    }

    // Assertions
    assert_eq!(errors.len(), 2, "should fail twice before succeeding");
    assert_eq!(
        final_output.as_deref(),
        Some("ok"),
        "final output should succeed on retry"
    );

    let attempts_data = fs::read_to_string(&attempts_log)?;
    let attempts_count = attempts_data.lines().count();
    assert_eq!(attempts_count, 3, "expected exactly 3 attempts recorded");

    env.down().await?;
    Ok(())
}

fn flaky_tool(log_path: &PathBuf, success_after: usize) -> Result<Result<String, String>> {
    let count_path = log_path.with_file_name("attempt_count.txt");
    let current = if count_path.exists() {
        let txt = fs::read_to_string(&count_path)
            .with_context(|| format!("failed to read {}", count_path.display()))?;
        txt.trim().parse::<usize>().unwrap_or(0)
    } else {
        0
    };
    let next = current + 1;
    fs::write(&count_path, next.to_string())
        .with_context(|| format!("failed to write {}", count_path.display()))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("failed to open {}", log_path.display()))?;
    let entry = json!({
        "attempt": next,
        "timestamp_ms": chrono::Utc::now().timestamp_millis(),
        "result": if next < success_after { "error" } else { "ok" }
    });
    writeln!(file, "{}", serde_json::to_string(&entry)?)
        .with_context(|| format!("failed to write {}", log_path.display()))?;

    if next < success_after {
        Ok(Err(format!("flaky tool error on attempt {}", next)))
    } else {
        Ok(Ok("ok".into()))
    }
}
