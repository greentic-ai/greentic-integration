use std::collections::BTreeSet;
use std::fs::{self, File};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use tempfile::tempdir;

#[path = "support/mod.rs"]
mod support;

const MCP_API_KEY_FALLBACK: &str = "6bb60b5ee66c4b25aff63044262402";
const CAPITAL_CITIES: [(&str, &str); 6] = [
    ("Nairobi", "Kenya"),
    ("Jakarta", "Indonesia"),
    ("Paris", "France"),
    ("Tokyo", "Japan"),
    ("Ottawa", "Canada"),
    ("Canberra", "Australia"),
];

struct WeatherCase {
    operation: &'static str,
    city: &'static str,
    country: &'static str,
    days: &'static str,
    api_node: &'static str,
    card_node: &'static str,
    expected_markers: &'static [&'static str],
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct ConversationResponse {
    #[serde(rename = "conversationId")]
    conversation_id: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct ActivitiesResponse {
    activities: Vec<Value>,
}

struct WebchatSession {
    conversation_id: String,
    token: String,
}

struct ServerGuard {
    child: Child,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn e2e_mcp_pack_smoke() -> Result<()> {
    let ci = is_ci();
    let local_opt_in = std::env::var("GREENTIC_MCP_SMOKE").ok().as_deref() == Some("1");
    if !ci && !local_opt_in {
        eprintln!("skipping e2e_mcp_pack_smoke: runs in CI or with GREENTIC_MCP_SMOKE=1");
        return Ok(());
    }

    support::init_test_logging();

    let api_key = std::env::var("MCP_API_KEY").unwrap_or_else(|_| MCP_API_KEY_FALLBACK.to_string());
    let repo_root = repo_root()?;
    let bundle_template = resolve_bundle_template_root(&repo_root)?;
    let tmp = tempdir().context("tempdir")?;
    let bundle_root = tmp.path().join("weather-mcp-demo-bundle");
    prepare_bundle(&bundle_template, &bundle_root, &api_key)?;

    let greentic_start_root = repo_root.join("greentic-start");
    let greentic_start = ensure_start_binary(&greentic_start_root)?;
    let port = pick_free_port()?;
    let _server = start_demo_server(&greentic_start, &greentic_start_root, &bundle_root, port)?;
    let base_url = format!("http://127.0.0.1:{port}");
    wait_for_server_ready(&base_url)?;
    let token = create_webchat_token(&base_url)?;

    for case in test_cases() {
        run_weather_case(&bundle_root, &base_url, &token, &case)?;
    }

    Ok(())
}

fn test_cases() -> Vec<WeatherCase> {
    let mut cases = Vec::with_capacity(CAPITAL_CITIES.len() * 2);
    for (city, country) in CAPITAL_CITIES {
        cases.push(WeatherCase {
            operation: "get_weather",
            city,
            country,
            days: "3",
            api_node: "call_weather",
            card_node: "render_current_card",
            expected_markers: &["Location:", "Temp ", "Feels ", "Wind "],
        });
        cases.push(WeatherCase {
            operation: "get_forecast_weather",
            city,
            country,
            days: "3",
            api_node: "call_forecast",
            card_node: "render_forecast_card",
            expected_markers: &["Location:", "Today ", "High ", "Low ", "Chance of rain"],
        });
    }
    cases
}

fn run_weather_case(
    bundle_root: &Path,
    base_url: &str,
    token: &str,
    case: &WeatherCase,
) -> Result<()> {
    let session = open_webchat_session(base_url, token)?;
    assert_initial_weather_card(base_url, &session)?;
    let location_marker = format!("Location: {}, {}", case.city, case.country);
    let existing_runs = list_run_dirs(bundle_root)?;
    post_activity(
        base_url,
        &session.conversation_id,
        &session.token,
        &json!({
            "type": "message",
            "from": { "id": "tester" },
            "text": format!("message {} {}", case.operation, case.city),
            "channelData": { "postBack": true },
            "value": {
                "operation": case.operation,
                "method": case.operation,
                "q": case.city,
                "days": case.days
            }
        }),
    )
    .with_context(|| format!("post weather activity for {} {}", case.operation, case.city))?;

    let result_activity = poll_for_activity(
        base_url,
        &session.conversation_id,
        &session.token,
        |activity| {
            let dump = serde_json::to_string(activity).unwrap_or_default();
            dump.contains(&location_marker)
                && case
                    .expected_markers
                    .iter()
                    .all(|marker| dump.contains(marker))
        },
    )?;
    let result_dump = serde_json::to_string_pretty(&result_activity)?;
    ensure!(
        result_dump.contains(case.city),
        "{} response dump does not contain city `{}`\n{}",
        case.operation,
        case.city,
        result_dump,
    );
    ensure!(
        result_dump.contains(case.country),
        "{} response dump does not contain country `{}`\n{}",
        case.operation,
        case.country,
        result_dump,
    );
    for marker in case.expected_markers {
        ensure!(
            result_dump.contains(marker),
            "{} response dump does not contain marker `{}`\n{}",
            case.operation,
            marker,
            result_dump,
        );
    }

    let run_dir = wait_for_new_run(bundle_root, &existing_runs, case)?;
    assert_transcript_matches(&run_dir, case)?;
    Ok(())
}

fn open_webchat_session(base_url: &str, token: &str) -> Result<WebchatSession> {
    let conversation = create_conversation(base_url, &token)?;
    Ok(WebchatSession {
        conversation_id: conversation.conversation_id,
        token: conversation.token,
    })
}

fn assert_initial_weather_card(base_url: &str, session: &WebchatSession) -> Result<()> {
    post_activity(
        base_url,
        &session.conversation_id,
        &session.token,
        &json!({
            "type": "message",
            "from": { "id": "tester" },
            "text": "hi"
        }),
    )?;

    let initial_card = poll_for_activity(
        base_url,
        &session.conversation_id,
        &session.token,
        |activity| {
            let dump = serde_json::to_string(activity).unwrap_or_default();
            dump.contains("Weather Assistant")
        },
    )?;
    let initial_dump = serde_json::to_string_pretty(&initial_card)?;
    ensure!(
        initial_dump.contains("Current Weather"),
        "initial weather card is missing current-weather action\n{}",
        initial_dump,
    );
    ensure!(
        initial_dump.contains("Forecast Weather"),
        "initial weather card is missing forecast action\n{}",
        initial_dump,
    );
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    ensure!(
        root.exists(),
        "workspace root missing at {}",
        root.display()
    );
    Ok(root)
}

fn resolve_bundle_template_root(repo_root: &Path) -> Result<PathBuf> {
    let root = repo_root.join("greentic-demo/weather-mcp-demo-bundle");
    ensure!(
        root.exists(),
        "weather demo bundle missing at {}",
        root.display()
    );
    Ok(root)
}

fn prepare_bundle(template_root: &Path, bundle_root: &Path, api_key: &str) -> Result<()> {
    copy_dir_all(template_root, bundle_root)?;

    let operator_log = bundle_root.join("logs/operator.log");
    if operator_log.exists() {
        fs::remove_file(&operator_log)
            .with_context(|| format!("remove {}", operator_log.display()))?;
    }

    let runs_dir = bundle_root.join("state/runs");
    if runs_dir.exists() {
        fs::remove_dir_all(&runs_dir).with_context(|| format!("remove {}", runs_dir.display()))?;
    }

    let setup_answers = bundle_root.join("state/config/weatherapi-pack/setup-answers.json");
    let answers = json!({
        "auth_param_get_weather_key": api_key,
        "auth_param_get_forecast_weather_key": api_key
    });
    fs::write(&setup_answers, serde_json::to_vec_pretty(&answers)?)
        .with_context(|| format!("write {}", setup_answers.display()))?;

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("create {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("read dir {}", src.display()))? {
        let entry = entry?;
        let entry_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry_path, &dst_path)?;
        } else {
            fs::copy(&entry_path, &dst_path).with_context(|| {
                format!("copy {} -> {}", entry_path.display(), dst_path.display())
            })?;
        }
    }
    Ok(())
}

fn ensure_start_binary(greentic_start_root: &Path) -> Result<PathBuf> {
    let binary = greentic_start_root.join("target/debug/greentic-start");
    if binary.exists() {
        return Ok(binary);
    }

    let status = Command::new("cargo")
        .args(["build", "-p", "greentic-start"])
        .current_dir(greentic_start_root)
        .status()
        .context("build greentic-start")?;
    ensure!(status.success(), "cargo build -p greentic-start failed");
    ensure!(
        binary.exists(),
        "greentic-start binary missing at {}",
        binary.display()
    );
    Ok(binary)
}

fn pick_free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind free port")?;
    let port = listener.local_addr().context("read local addr")?.port();
    drop(listener);
    Ok(port)
}

fn start_demo_server(
    binary: &Path,
    greentic_start_root: &Path,
    bundle_root: &Path,
    port: u16,
) -> Result<ServerGuard> {
    let stdout_path = bundle_root.join("logs/test-server.stdout.log");
    let stderr_path = bundle_root.join("logs/test-server.stderr.log");
    let stdout =
        File::create(&stdout_path).with_context(|| format!("create {}", stdout_path.display()))?;
    let stderr =
        File::create(&stderr_path).with_context(|| format!("create {}", stderr_path.display()))?;

    let child = Command::new(binary)
        .arg("start")
        .arg("--bundle")
        .arg(bundle_root)
        .arg("--cloudflared")
        .arg("off")
        .arg("--ngrok")
        .arg("off")
        .arg("--nats")
        .arg("off")
        .env("GREENTIC_GATEWAY_LISTEN_ADDR", "127.0.0.1")
        .env("GREENTIC_GATEWAY_PORT", port.to_string())
        .env("GREENTIC_ENV", "dev")
        .current_dir(greentic_start_root)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("spawn {}", binary.display()))?;

    Ok(ServerGuard { child })
}

fn wait_for_server_ready(base_url: &str) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    let token_url = format!("{base_url}/v1/messaging/webchat/demo/token?env=dev&tenant=demo");
    loop {
        match ureq::post(&token_url).send_json(json!({})) {
            Ok(response) => {
                if response.status() == 200 {
                    return Ok(());
                }
            }
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            anyhow::bail!("weather demo server did not become ready at {base_url}");
        }
        sleep(Duration::from_millis(500));
    }
}

fn create_webchat_token(base_url: &str) -> Result<String> {
    let url = format!("{base_url}/v1/messaging/webchat/demo/token?env=dev&tenant=demo");
    let mut response = ureq::post(&url)
        .send_json(json!({}))
        .with_context(|| format!("POST {url}"))?;
    let body: TokenResponse = response
        .body_mut()
        .read_json()
        .context("parse token response")?;
    Ok(body.token)
}

fn create_conversation(base_url: &str, token: &str) -> Result<ConversationResponse> {
    let url = format!(
        "{base_url}/v1/messaging/webchat/demo/v3/directline/conversations?env=dev&tenant=demo"
    );
    let mut response = ureq::post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send_json(json!({}))
        .with_context(|| format!("POST {url}"))?;
    response
        .body_mut()
        .read_json()
        .context("parse conversation response")
}

fn post_activity(base_url: &str, conversation_id: &str, token: &str, body: &Value) -> Result<()> {
    let url = format!(
        "{base_url}/v1/messaging/webchat/demo/v3/directline/conversations/{conversation_id}/activities?env=dev&tenant=demo"
    );
    let response = ureq::post(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .send_json(body)
        .with_context(|| format!("POST {url}"))?;
    let status = response.status().as_u16();
    ensure!(
        (200..300).contains(&status),
        "activity post failed with status {status}"
    );
    Ok(())
}

fn poll_for_activity<F>(
    base_url: &str,
    conversation_id: &str,
    token: &str,
    predicate: F,
) -> Result<Value>
where
    F: Fn(&Value) -> bool,
{
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let activities = fetch_activities(base_url, conversation_id, token)?;
        let last_dump = serde_json::to_string_pretty(&activities).unwrap_or_default();
        if let Some(activity) = activities.into_iter().find(|activity| predicate(activity)) {
            return Ok(activity);
        }
        if Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for matching activity\n{last_dump}");
        }
        sleep(Duration::from_millis(750));
    }
}

fn fetch_activities(base_url: &str, conversation_id: &str, token: &str) -> Result<Vec<Value>> {
    let url = format!(
        "{base_url}/v1/messaging/webchat/demo/v3/directline/conversations/{conversation_id}/activities?env=dev&tenant=demo"
    );
    let mut response = ureq::get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
        .with_context(|| format!("GET {url}"))?;
    let body: ActivitiesResponse = response
        .body_mut()
        .read_json()
        .context("parse activities")?;
    Ok(body.activities)
}

fn list_run_dirs(bundle_root: &Path) -> Result<BTreeSet<String>> {
    let runs_root = bundle_root.join("state/runs/messaging/weatherapi-pack/default");
    if !runs_root.exists() {
        return Ok(BTreeSet::new());
    }
    let mut dirs = BTreeSet::new();
    for entry in
        fs::read_dir(&runs_root).with_context(|| format!("read dir {}", runs_root.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.insert(entry.file_name().to_string_lossy().to_string());
        }
    }
    Ok(dirs)
}

fn wait_for_new_run(
    bundle_root: &Path,
    existing_runs: &BTreeSet<String>,
    case: &WeatherCase,
) -> Result<PathBuf> {
    let runs_root = bundle_root.join("state/runs/messaging/weatherapi-pack/default");
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        for run_name in list_run_dirs(bundle_root)? {
            if existing_runs.contains(&run_name) {
                continue;
            }
            let run_dir = runs_root.join(&run_name);
            let transcript_path = run_dir.join("transcript.jsonl");
            if !transcript_path.exists() {
                continue;
            }
            let transcript = fs::read_to_string(&transcript_path)
                .with_context(|| format!("read {}", transcript_path.display()))?;
            let entries = parse_transcript(&transcript)?;
            if transcript_entry(&entries, case.api_node, "end").is_some()
                && transcript_entry(&entries, case.card_node, "end").is_some()
            {
                return Ok(run_dir);
            }
        }
        if Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for transcript run for {} under {}",
                case.operation,
                runs_root.display()
            );
        }
        sleep(Duration::from_millis(500));
    }
}

fn assert_transcript_matches(run_dir: &Path, case: &WeatherCase) -> Result<()> {
    let transcript_path = run_dir.join("transcript.jsonl");
    let transcript = fs::read_to_string(&transcript_path)
        .with_context(|| format!("read {}", transcript_path.display()))?;
    let entries = parse_transcript(&transcript)?;

    let api_entry = transcript_entry(&entries, case.api_node, "end")
        .with_context(|| format!("missing API transcript entry for {}", case.api_node))?;
    let structured = api_entry
        .pointer("/outputs/result/structured_content")
        .cloned()
        .context("structured_content missing from API transcript entry")?;
    let location_name = structured
        .pointer("/location/name")
        .and_then(Value::as_str)
        .context("location.name missing from structured content")?;
    ensure!(
        location_name.eq_ignore_ascii_case(case.city),
        "{} returned city `{}` instead of `{}`\nstructured_content:\n{}",
        case.operation,
        location_name,
        case.city,
        serde_json::to_string_pretty(&structured)?,
    );
    ensure!(
        structured
            .pointer("/location/country")
            .and_then(Value::as_str)
            == Some(case.country),
        "{} returned unexpected country\nstructured_content:\n{}",
        case.operation,
        serde_json::to_string_pretty(&structured)?,
    );

    match case.operation {
        "get_weather" => ensure!(
            structured.get("current").is_some(),
            "current weather payload missing `current`\nstructured_content:\n{}",
            serde_json::to_string_pretty(&structured)?,
        ),
        "get_forecast_weather" => ensure!(
            structured.get("forecast").is_some(),
            "forecast weather payload missing `forecast`\nstructured_content:\n{}",
            serde_json::to_string_pretty(&structured)?,
        ),
        other => anyhow::bail!("unexpected operation {other}"),
    }

    let card_entry = transcript_entry(&entries, case.card_node, "end")
        .with_context(|| format!("missing card transcript entry for {}", case.card_node))?;
    let rendered_card = card_entry
        .pointer("/outputs/renderedCard")
        .cloned()
        .context("renderedCard missing from card transcript entry")?;
    let card_dump = serde_json::to_string_pretty(&rendered_card)?;
    let location_marker = format!("Location: {}, {}", case.city, case.country);
    ensure!(
        card_dump.contains(&location_marker),
        "{} card does not contain expected city marker `{}`\ncard dump:\n{}",
        case.operation,
        location_marker,
        card_dump,
    );
    for marker in case.expected_markers {
        ensure!(
            card_dump.contains(marker),
            "{} card missing marker `{}`\ncard dump:\n{}",
            case.operation,
            marker,
            card_dump,
        );
    }

    Ok(())
}

fn parse_transcript(transcript: &str) -> Result<Vec<Value>> {
    transcript
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).context("parse transcript line"))
        .collect()
}

fn transcript_entry<'a>(entries: &'a [Value], node_id: &str, phase: &str) -> Option<&'a Value> {
    entries.iter().find(|entry| {
        entry.get("node_id").and_then(Value::as_str) == Some(node_id)
            && entry.get("phase").and_then(Value::as_str) == Some(phase)
    })
}

fn is_ci() -> bool {
    std::env::var("CI")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
