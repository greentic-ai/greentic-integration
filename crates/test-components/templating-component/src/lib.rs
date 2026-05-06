use serde_json::{Value, json};

// Compat types matching the old greentic:component/node@0.5.0 shapes.
// The published greentic-interfaces-guest >=1.1 removed the 0.5.0 surface
// (component-node feature → component-v0-6). These test fixtures only need
// the type definitions for host-side unit tests; WASM export glue is handled
// separately via wit_bindgen.
#[derive(Debug)]
pub enum InvokeResult {
    Ok(String),
    Err(NodeError),
}

#[derive(Debug)]
pub struct NodeError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub backoff_ms: Option<u64>,
    pub details: Option<String>,
}

#[allow(dead_code)]
fn manifest() -> String {
    json!({
        "id": "conformance.templating",
        "version": "0.1.0",
        "world": "greentic:component/component@0.5.0",
        "supports": ["job"],
        "profiles": {"default": "default", "supported": ["default"]},
        "capabilities": {"wasi": {}, "host": {}},
        "operations": [
            {"name": "start", "input_schema": {}, "output_schema": {}},
            {"name": "process", "input_schema": {}, "output_schema": {}}
        ]
    })
    .to_string()
}

#[allow(dead_code)]
fn invoke(op: String, input: String) -> InvokeResult {
    match op.as_str() {
        "start" => InvokeResult::Ok(
            json!({
                "user": {"id": 1, "name": "Ada"},
                "status": "ready"
            })
            .to_string(),
        ),
        "process" => {
            let payload: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
            let user_id = payload.get("user_id").cloned().unwrap_or(Value::Null);
            let user_id_type = if user_id.is_number() {
                "number"
            } else if user_id.is_string() {
                "string"
            } else {
                "other"
            };
            let name = payload.get("name").cloned().unwrap_or(Value::Null);
            let status = payload.get("status").cloned().unwrap_or(Value::Null);
            let message = payload.get("message").cloned().unwrap_or(Value::Null);
            InvokeResult::Ok(
                json!({
                    "marker": "templating.process",
                    "user_id": user_id,
                    "user_id_type": user_id_type,
                    "name": name,
                    "status": status,
                    "message": message
                })
                .to_string(),
            )
        }
        _ => InvokeResult::Err(NodeError {
            code: "unsupported_operation".to_string(),
            message: format!("unsupported op: {op}"),
            retryable: false,
            backoff_ms: None,
            details: None,
        }),
    }
}

// NOTE: the old component_entrypoint! macro (0.5.0 ABI glue) was removed
// from greentic-interfaces-guest >=1.1. WASM export glue for these test
// fixtures will be revisited when the integration harness migrates to the
// 0.6.0 invoke contract.
