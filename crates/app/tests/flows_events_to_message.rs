use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;
use serde_yaml_bw as serde_yaml;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Flow {
    #[serde(rename = "type")]
    flow_type: String,
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    nodes: HashMap<String, NodeDefinition>,
}

#[derive(Debug, Deserialize)]
struct NodeDefinition {
    #[serde(flatten)]
    operations: HashMap<String, OperatorConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OperatorConfig {
    component: Option<String>,
    profile: Option<String>,
    provider: Option<String>,
    channel: Option<String>,
    topic: Option<String>,
    #[serde(default)]
    config: Value,
    #[serde(default)]
    routing: HashMap<String, String>,
}

fn load_flow(relative_path: &str) -> Flow {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let flow_path = manifest_dir.join("..").join("..").join(relative_path);
    let data = fs::read_to_string(&flow_path)
        .unwrap_or_else(|_| panic!("Failed to read flow file at {:?}", flow_path));
    serde_yaml::from_str(&data).expect("flow YAML should deserialize")
}

#[test]
fn build_status_flow_structure_is_valid() {
    let flow = load_flow("flows/events_to_message/build_status_notifications.ygtc");

    assert_eq!(flow.flow_type, "events");
    assert_eq!(flow.id, "build_status_notifications");

    let ingress = flow
        .nodes
        .get("event_ingress")
        .expect("event_ingress node present");
    let ingress_op = ingress
        .operations
        .get("events.source")
        .expect("events.source operator present");
    assert_eq!(ingress_op.provider.as_deref(), Some("local-events"));
    assert_eq!(
        ingress_op.topic.as_deref(),
        Some("greentic.repo.build.status")
    );
    assert_eq!(
        ingress_op.routing.get("default").map(String::as_str),
        Some("bridge_to_message")
    );

    let bridge = flow
        .nodes
        .get("bridge_to_message")
        .expect("bridge_to_message node present");
    let bridge_op = bridge
        .operations
        .get("events.bridge.message_to_channel")
        .expect("message bridge operator present");
    assert_eq!(
        bridge_op.component.as_deref(),
        Some("demo.bridge.events_to_message")
    );
    assert_eq!(
        bridge_op.routing.get("default").map(String::as_str),
        Some("send_message")
    );

    let send = flow
        .nodes
        .get("send_message")
        .expect("send_message node present");
    let send_op = send
        .operations
        .get("messaging.send")
        .expect("messaging.send operator present");
    assert_eq!(send_op.provider.as_deref(), Some("local-messaging"));
    assert_eq!(
        send_op.routing.get("default").map(String::as_str),
        Some("done")
    );

    assert!(
        flow.nodes.contains_key("done"),
        "done terminal node should exist"
    );
}
