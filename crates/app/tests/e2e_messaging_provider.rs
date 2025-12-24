use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::Client;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use futures::StreamExt;
use greentic_integration::harness::{TestEnv, docker_available};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;

/// E2E messaging/provider flow smoke suite.
///
/// Spins up the docker-compose test stack for NATS, runs a tiny NATS-driven "flow worker"
/// that forwards payloads to a stub provider sink (HTTP), and asserts the captured outbound
/// JSON artifacts match expectations (text transform, thread continuity, adaptive cards).
static DOCKER_TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn ensure_docker(test: &str) -> anyhow::Result<bool> {
    if docker_available() {
        return Ok(true);
    }
    let strict = std::env::var("E2E_REQUIRE_DOCKER")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if strict {
        anyhow::bail!("{test} requires docker but daemon is unavailable");
    }
    eprintln!("{test}: skipping, docker daemon not available");
    Ok(false)
}

#[tokio::test]
async fn e2e_messaging_provider_nats_smoke() -> anyhow::Result<()> {
    let _guard = DOCKER_TEST_LOCK.lock().await;

    if !ensure_docker("e2e_messaging_provider_nats_smoke")? {
        return Ok(());
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    let nats_url = env.nats_url();
    let subject = "e2e.messaging.provider.smoke";
    let payload = json!({"ping": true});

    let client = async_nats::connect(&nats_url)
        .await
        .with_context(|| format!("connect to NATS at {}", nats_url))?;
    let mut sub = client.subscribe(subject.to_string()).await?;
    client
        .publish(subject.to_string(), serde_json::to_vec(&payload)?.into())
        .await?;
    client.flush().await?;

    let msg = timeout(Duration::from_secs(5), sub.next())
        .await
        .context("timed out awaiting pub/sub smoke message")?
        .ok_or_else(|| anyhow::anyhow!("subscription ended before message"))?;
    let received: Value = serde_json::from_slice(&msg.payload)?;
    assert_eq!(received, payload, "NATS pub/sub smoke payload mismatch");

    env.down().await?;
    Ok(())
}

/// Full provider flow coverage.
#[tokio::test]
async fn e2e_messaging_provider_flow() -> anyhow::Result<()> {
    let _guard = DOCKER_TEST_LOCK.lock().await;

    if !ensure_docker("e2e_messaging_provider_flow")? {
        return Ok(());
    }

    let env = TestEnv::up().await?;
    env.healthcheck().await?;

    let provider = "stub-provider".to_string();

    // 1) simple text roundtrip (hello -> HELLO)
    let text_payload = run_case(
        &env,
        "text_roundtrip",
        FlowBehavior::Uppercase {
            provider: provider.clone(),
        },
        InboundMessage {
            text: Some("hello".into()),
            ..Default::default()
        },
    )
    .await?;
    assert_eq!(text_payload["text"], "HELLO");

    // 2) reply/thread continuity
    let reply_payload = run_case(
        &env,
        "reply_thread",
        FlowBehavior::ThreadContinuity {
            provider: provider.clone(),
        },
        InboundMessage {
            text: Some("ping".into()),
            thread_id: Some("thread-123".into()),
            reply_to: Some("msg-999".into()),
        },
    )
    .await?;
    assert_eq!(reply_payload["thread_id"], "thread-123");
    assert_eq!(reply_payload["reply_to"], "msg-999");

    // 3) basic adaptive card preserved end-to-end
    let basic_card = run_case(
        &env,
        "adaptive_basic",
        FlowBehavior::Card {
            provider: provider.clone(),
            card: CardKind::Basic,
        },
        InboundMessage {
            text: Some("card please".into()),
            ..Default::default()
        },
    )
    .await?;
    assert_eq!(basic_card["card"]["type"], "AdaptiveCard");
    assert_eq!(basic_card["card"]["version"], "1.5");
    assert_eq!(
        basic_card["card"]["body"][0]["text"],
        "Here is a basic card"
    );

    // 4) adaptive card with inputs and submit action
    let input_card = run_case(
        &env,
        "adaptive_inputs",
        FlowBehavior::Card {
            provider: provider.clone(),
            card: CardKind::Inputs,
        },
        InboundMessage {
            text: Some("collect inputs".into()),
            ..Default::default()
        },
    )
    .await?;
    let body = input_card["card"]["body"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        body.iter()
            .any(|entry| entry["id"] == "preference" && entry["type"] == "Input.ChoiceSet"),
        "expected preference choice set preserved"
    );
    assert_eq!(
        input_card["card"]["actions"][0]["title"],
        "Submit preferences"
    );

    // 5) provider translation smoke across variants (ensures top-level and URL fields intact)
    for provider_name in ["teams", "slack", "webchat"] {
        let smoke = run_case(
            &env,
            &format!("provider_smoke_{provider_name}"),
            FlowBehavior::ProviderSmoke {
                provider: provider_name.to_string(),
            },
            InboundMessage {
                text: Some("smoke".into()),
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(smoke["provider"], provider_name);
        assert_eq!(smoke["card"]["type"], "AdaptiveCard");
        assert_eq!(
            smoke["card"]["actions"][0]["url"],
            "https://example.com/docs"
        );
    }

    // 6) multi-message session continuity.
    let continuity_payloads = run_sequence(
        &env,
        "session_continuity",
        FlowBehavior::ThreadContinuity {
            provider: provider.clone(),
        },
        vec![
            InboundMessage {
                text: Some("first".into()),
                thread_id: Some("thread-seq".into()),
                reply_to: Some("m0".into()),
            },
            InboundMessage {
                text: Some("second".into()),
                thread_id: Some("thread-seq".into()),
                reply_to: Some("m1".into()),
            },
        ],
    )
    .await?;
    assert_eq!(
        continuity_payloads.len(),
        2,
        "expected two outbound payloads"
    );
    assert_eq!(continuity_payloads[0]["text"], "first");
    assert_eq!(continuity_payloads[1]["text"], "second");
    for payload in &continuity_payloads {
        assert_eq!(payload["thread_id"], "thread-seq");
    }

    // 7) oversize message still delivered (no crash).
    let oversized = "x".repeat(8 * 1024);
    let oversize_payload = run_case(
        &env,
        "oversize_message",
        FlowBehavior::ThreadContinuity {
            provider: provider.clone(),
        },
        InboundMessage {
            text: Some(oversized.clone()),
            thread_id: Some("thread-big".into()),
            reply_to: None,
        },
    )
    .await?;
    assert_eq!(oversize_payload["text"], oversized);
    assert_eq!(oversize_payload["thread_id"], "thread-big");

    // 8) Basic shape check for a few captured payloads.
    for (case, payload) in [
        ("text_roundtrip", text_payload.clone()),
        ("reply_thread", reply_payload.clone()),
        ("adaptive_basic", basic_card.clone()),
    ] {
        assert!(
            payload.get("provider").is_some(),
            "{case}: missing provider field"
        );
        if payload.get("card").is_none() {
            assert!(payload.get("text").is_some(), "{case}: missing text");
        }
    }

    // 9) sink resilience: slow sink still succeeds; error sink surfaces clean failure.
    let slow_payload = run_case_with_mode(
        &env,
        "slow_sink",
        FlowBehavior::ThreadContinuity {
            provider: provider.clone(),
        },
        InboundMessage {
            text: Some("slow".into()),
            thread_id: Some("thread-slow".into()),
            reply_to: None,
        },
        ResponseMode::OkSlow { delay_ms: 1500 },
    )
    .await?;
    assert_eq!(slow_payload["text"], "slow");

    let err = run_case_with_mode(
        &env,
        "error_sink",
        FlowBehavior::ThreadContinuity {
            provider: provider.clone(),
        },
        InboundMessage {
            text: Some("boom".into()),
            thread_id: Some("thread-err".into()),
            reply_to: None,
        },
        ResponseMode::Error {
            status: StatusCode::INTERNAL_SERVER_ERROR,
        },
    )
    .await;
    assert!(
        err.is_err(),
        "error sink should surface failure instead of succeeding"
    );

    env.down().await?;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct InboundMessage {
    text: Option<String>,
    thread_id: Option<String>,
    reply_to: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OutboundPayload {
    provider: String,
    text: Option<String>,
    thread_id: Option<String>,
    reply_to: Option<String>,
    card: Option<Value>,
}

#[derive(Clone, Copy)]
enum CardKind {
    Basic,
    Inputs,
}

#[derive(Clone)]
enum FlowBehavior {
    Uppercase { provider: String },
    ThreadContinuity { provider: String },
    Card { provider: String, card: CardKind },
    ProviderSmoke { provider: String },
}

impl FlowBehavior {
    fn apply(&self, inbound: InboundMessage) -> OutboundPayload {
        match self {
            FlowBehavior::Uppercase { provider } => OutboundPayload {
                provider: provider.clone(),
                text: inbound.text.map(|t| t.to_ascii_uppercase()),
                thread_id: inbound.thread_id,
                reply_to: inbound.reply_to,
                card: None,
            },
            FlowBehavior::ThreadContinuity { provider } => OutboundPayload {
                provider: provider.clone(),
                text: inbound.text,
                thread_id: inbound.thread_id,
                reply_to: inbound.reply_to,
                card: None,
            },
            FlowBehavior::Card { provider, card } => OutboundPayload {
                provider: provider.clone(),
                text: inbound.text,
                thread_id: inbound.thread_id,
                reply_to: inbound.reply_to,
                card: Some(match card {
                    CardKind::Basic => basic_card(),
                    CardKind::Inputs => inputs_card(),
                }),
            },
            FlowBehavior::ProviderSmoke { provider } => OutboundPayload {
                provider: provider.clone(),
                text: inbound.text,
                thread_id: inbound.thread_id,
                reply_to: inbound.reply_to,
                card: Some(provider_smoke_card()),
            },
        }
    }
}

fn basic_card() -> Value {
    json!({
        "type": "AdaptiveCard",
        "version": "1.5",
        "body": [
            { "type": "TextBlock", "size": "Medium", "weight": "Bolder", "text": "Here is a basic card" },
            { "type": "TextBlock", "wrap": true, "text": "Static content to validate payload preservation." }
        ]
    })
}

fn inputs_card() -> Value {
    json!({
        "type": "AdaptiveCard",
        "version": "1.5",
        "body": [
            { "type": "TextBlock", "text": "Pick a preference", "wrap": true },
            {
                "type": "Input.ChoiceSet",
                "id": "preference",
                "style": "expanded",
                "choices": [
                    { "title": "Email", "value": "email" },
                    { "title": "SMS", "value": "sms" }
                ]
            },
            { "type": "Input.Text", "id": "notes", "placeholder": "Optional notes" }
        ],
        "actions": [
            { "type": "Action.Submit", "title": "Submit preferences", "data": { "action": "save_preferences" } }
        ]
    })
}

fn provider_smoke_card() -> Value {
    json!({
        "type": "AdaptiveCard",
        "version": "1.5",
        "body": [
            { "type": "TextBlock", "text": "Provider smoke test", "wrap": true },
            { "type": "TextBlock", "text": "Ensure URL + inputs survive translation." }
        ],
        "actions": [
            { "type": "Action.OpenUrl", "title": "Open docs", "url": "https://example.com/docs" }
        ]
    })
}

async fn run_case(
    env: &TestEnv,
    case: &str,
    behavior: FlowBehavior,
    inbound: InboundMessage,
) -> anyhow::Result<Value> {
    run_case_with_mode(env, case, behavior, inbound, ResponseMode::OkFast).await
}

async fn run_case_with_mode(
    env: &TestEnv,
    case: &str,
    behavior: FlowBehavior,
    inbound: InboundMessage,
    mode: ResponseMode,
) -> anyhow::Result<Value> {
    let artifacts = env.artifacts_dir().join("provider-e2e").join(case);
    let sink = ProviderSink::start_with_mode(artifacts.join("outbound.json"), mode).await?;

    let subject = format!("e2e.messaging.{case}");
    let mut worker = FlowWorker::spawn(
        env.nats_url(),
        subject.clone(),
        format!("{}/send", sink.url()),
        behavior,
        1,
    );

    worker.wait_ready(Duration::from_secs(5)).await?;
    publish(env.nats_url(), &subject, &inbound).await?;
    worker.wait(Duration::from_secs(5)).await?;

    let captured = sink.wait_for(1, Duration::from_secs(8)).await?;
    sink.shutdown().await?;

    captured
        .into_iter()
        .last()
        .ok_or_else(|| anyhow::anyhow!("no outbound payload captured"))
}

async fn run_sequence(
    env: &TestEnv,
    case: &str,
    behavior: FlowBehavior,
    inbound_msgs: Vec<InboundMessage>,
) -> anyhow::Result<Vec<Value>> {
    let artifacts = env.artifacts_dir().join("provider-e2e").join(case);
    let sink = ProviderSink::start_with_mode(artifacts.join("outbound.json"), ResponseMode::OkFast)
        .await?;

    let subject = format!("e2e.messaging.{case}");
    let mut worker = FlowWorker::spawn(
        env.nats_url(),
        subject.clone(),
        format!("{}/send", sink.url()),
        behavior,
        inbound_msgs.len(),
    );

    worker.wait_ready(Duration::from_secs(5)).await?;
    for msg in inbound_msgs {
        publish(env.nats_url(), &subject, &msg).await?;
    }
    let expected = worker.expected;
    worker.wait(Duration::from_secs(10)).await?;

    let captured = sink.wait_for(expected, Duration::from_secs(8)).await?;
    sink.shutdown().await?;
    Ok(captured)
}

async fn publish(nats_url: String, subject: &str, inbound: &InboundMessage) -> anyhow::Result<()> {
    let client = async_nats::connect(nats_url)
        .await
        .with_context(|| "connect to NATS")?;
    client
        .publish(subject.to_string(), serde_json::to_vec(inbound)?.into())
        .await?;
    client.flush().await?;
    Ok(())
}

struct FlowWorker {
    handle: JoinHandle<anyhow::Result<()>>,
    ready: oneshot::Receiver<()>,
    expected: usize,
}

impl FlowWorker {
    fn spawn(
        nats_url: String,
        subject: String,
        sink_url: String,
        behavior: FlowBehavior,
        expected: usize,
    ) -> Self {
        let (ready_tx, ready_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            let client: Client = async_nats::connect(&nats_url)
                .await
                .with_context(|| format!("connect to NATS at {}", nats_url))?;
            let mut sub = client.subscribe(subject.clone()).await?;
            let _ = ready_tx.send(());
            for idx in 0..expected {
                let msg = timeout(Duration::from_secs(20), sub.next())
                    .await
                    .with_context(|| {
                        format!("timed out awaiting inbound message {idx} (subscribe->next)")
                    })?
                    .ok_or_else(|| anyhow::anyhow!("subscription ended before message"))?;
                let inbound: InboundMessage = serde_json::from_slice(&msg.payload)?;
                let outbound = behavior.apply(inbound);
                send_to_sink(&sink_url, &outbound).await?;
            }
            Ok(())
        });
        Self {
            handle,
            ready: ready_rx,
            expected,
        }
    }

    async fn wait_ready(&mut self, timeout_dur: Duration) -> anyhow::Result<()> {
        timeout(timeout_dur, &mut self.ready)
            .await
            .context("timed out waiting for worker subscribe ready")?
            .map_err(|_| anyhow::anyhow!("worker subscribe channel closed"))
    }

    async fn wait(self, _timeout_dur: Duration) -> anyhow::Result<()> {
        let join = self.handle.await;
        match join {
            Ok(res) => res,
            Err(err) => Err(anyhow::anyhow!("worker task join error: {err}")),
        }
    }
}

async fn send_to_sink(url: &str, outbound: &OutboundPayload) -> anyhow::Result<()> {
    let url = url.to_string();
    let outbound = outbound.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let body = serde_json::to_value(&outbound)?;
        let resp = ureq::post(&url).send_json(body);
        match resp {
            Ok(r) if r.status() == StatusCode::OK.as_u16() => Ok(()),
            Ok(r) => anyhow::bail!("sink responded with {}", r.status()),
            Err(err) => anyhow::bail!("failed to POST to sink: {err}"),
        }
    })
    .await
    .expect("spawn_blocking failed")
}

struct ProviderSink {
    url: String,
    state: Arc<SinkState>,
    shutdown: Option<oneshot::Sender<()>>,
    handle: JoinHandle<()>,
}

struct SinkState {
    path: PathBuf,
    entries: Mutex<Vec<Value>>,
    mode: ResponseMode,
}

#[derive(Clone, Copy)]
enum ResponseMode {
    OkFast,
    OkSlow { delay_ms: u64 },
    Error { status: StatusCode },
}

impl ProviderSink {
    async fn start_with_mode(path: PathBuf, mode: ResponseMode) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let state = Arc::new(SinkState {
            path: path.clone(),
            entries: Mutex::new(Vec::new()),
            mode,
        });
        let router = Router::new()
            .route("/send", post(handle_sink))
            .with_state(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let url = format!("http://{addr}");

        let (tx, rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let server = axum::serve(listener, router.into_make_service());
            let _ = server
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await;
        });

        Ok(Self {
            url,
            state,
            shutdown: Some(tx),
            handle,
        })
    }

    fn url(&self) -> &str {
        &self.url
    }

    async fn wait_for(&self, expected: usize, timeout_dur: Duration) -> anyhow::Result<Vec<Value>> {
        let start = std::time::Instant::now();
        loop {
            let entries = { self.state.entries.lock().await.clone() };
            if entries.len() >= expected {
                return Ok(entries);
            }
            if start.elapsed() > timeout_dur {
                anyhow::bail!(
                    "timed out waiting for {} outbound payload(s); got {}",
                    expected,
                    entries.len()
                );
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn shutdown(mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
        Ok(())
    }
}

async fn handle_sink(
    State(state): State<Arc<SinkState>>,
    Json(payload): Json<Value>,
) -> StatusCode {
    match state.mode {
        ResponseMode::OkFast => record_and_status(&state, payload, StatusCode::OK).await,
        ResponseMode::OkSlow { delay_ms } => {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            record_and_status(&state, payload, StatusCode::OK).await
        }
        ResponseMode::Error { status } => record_and_status(&state, payload, status).await,
    }
}

async fn record_and_status(state: &SinkState, payload: Value, status: StatusCode) -> StatusCode {
    {
        let mut guard = state.entries.lock().await;
        guard.push(payload.clone());
        let serialized = serde_json::to_string_pretty(&*guard).unwrap_or_default();
        if let Some(parent) = state.path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&state.path, serialized).await;
    }
    status
}
