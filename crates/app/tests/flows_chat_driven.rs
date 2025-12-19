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
        .unwrap_or_else(|_| panic!("Failed to read flow file at {flow_path:?}"));
    serde_yaml::from_str(&data).expect("flow YAML should deserialize")
}

#[test]
fn chat_driven_flow_has_expected_nodes_and_routing() {
    let flow = load_flow("flows/chat_driven/repo_assistant.ygtc");

    assert_eq!(flow.flow_type, "messaging");
    assert_eq!(flow.id, "repo_assistant_chat");

    let ingress = flow
        .nodes
        .get("ingress_message")
        .expect("ingress_message node present");
    let ingress_op = ingress
        .operations
        .get("messaging.ingress")
        .expect("messaging.ingress operator present");
    assert_eq!(ingress_op.provider.as_deref(), Some("local-messaging"));
    assert_eq!(ingress_op.channel.as_deref(), Some("webchat"));
    assert_eq!(
        ingress_op.routing.get("default").map(String::as_str),
        Some("to_worker")
    );

    let worker = flow.nodes.get("to_worker").expect("to_worker node present");
    let worker_op = worker
        .operations
        .get("worker.request")
        .expect("worker.request operator present");
    assert_eq!(
        worker_op.component.as_deref(),
        Some("demo.worker.repo_assistant")
    );
    assert_eq!(
        worker_op.routing.get("default").map(String::as_str),
        Some("respond")
    );
    assert_eq!(
        worker_op
            .routing
            .get("rebuild_requested")
            .map(String::as_str),
        Some("emit_rebuild_event")
    );

    let respond = flow.nodes.get("respond").expect("respond node present");
    let respond_op = respond
        .operations
        .get("messaging.send")
        .expect("messaging.send operator present");
    assert_eq!(
        respond_op.routing.get("default").map(String::as_str),
        Some("done")
    );

    let emit_rebuild = flow
        .nodes
        .get("emit_rebuild_event")
        .expect("emit_rebuild_event node present");
    let emit_op = emit_rebuild
        .operations
        .get("events.publish")
        .expect("events.publish operator present");
    assert_eq!(
        emit_op.topic.as_deref(),
        Some("greentic.repo.build.request")
    );
    assert_eq!(
        emit_op.routing.get("default").map(String::as_str),
        Some("done")
    );
}
