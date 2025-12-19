use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn load_payload(name: &str) -> Value {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir
        .join("..")
        .join("..")
        .join("samples")
        .join("payloads")
        .join(name);
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read payload file at {path:?}"));
    serde_json::from_str(&data).expect("payload JSON should parse")
}

#[test]
fn build_status_event_payload_has_expected_fields() {
    let payload = load_payload("build_status_event.json");
    assert_eq!(payload["topic"], "greentic.repo.build.status");
    assert_eq!(payload["type"], "com.greentic.repo.build.status.v1");
    assert_eq!(payload["subject"], "repo:my-service");
    assert!(payload["tenant"].is_object());
    assert!(payload["payload"].is_object());
    assert!(payload["metadata"].is_object());
}

#[test]
fn channel_message_payload_has_expected_fields() {
    let payload = load_payload("channel_message.json");
    assert_eq!(payload["channel"], "webchat");
    assert_eq!(payload["session_id"], "sess-789");
    assert_eq!(
        payload["text"],
        "Build succeeded for my-service @1a2b3c (status: success)"
    );
    assert!(payload["tenant"].is_object());
    assert!(payload["attachments"].is_array());
    assert!(payload["metadata"].is_object());
}

#[test]
fn rebuild_request_event_payload_has_expected_fields() {
    let payload = load_payload("rebuild_request_event.json");
    assert_eq!(payload["topic"], "greentic.repo.build.request");
    assert_eq!(payload["type"], "com.greentic.repo.build.request.v1");
    assert_eq!(payload["subject"], "repo:my-service");
    assert!(payload["tenant"].is_object());
    assert!(payload["payload"].is_object());
    assert!(payload["metadata"].is_object());
}
