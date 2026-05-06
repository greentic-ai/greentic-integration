#![allow(dead_code)]

use greentic_interfaces_guest::state_store::read;
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

fn manifest() -> String {
    json!({
        "id": "conformance.state_nocap",
        "version": "0.1.0",
        "world": "greentic:component/component@0.5.0",
        "supports": ["job"],
        "profiles": {"default": "default", "supported": ["default"]},
        "capabilities": {"wasi": {}, "host": {}},
        "operations": [
            {"name": "touch", "input_schema": {}, "output_schema": {}}
        ]
    })
    .to_string()
}

fn invoke(op: String, input: String) -> InvokeResult {
    if op.as_str() != "touch" {
        return InvokeResult::Err(NodeError {
            code: "unsupported_operation".to_string(),
            message: format!("unsupported op: {op}"),
            retryable: false,
            backoff_ms: None,
            details: None,
        });
    }
    let payload: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    let key = payload
        .get("key")
        .and_then(Value::as_str)
        .unwrap_or("conformance-key")
        .to_string();
    match read(&key, None) {
        Ok(_) => InvokeResult::Ok(
            json!({"marker": "state.nocap", "status": "unexpected_access"}).to_string(),
        ),
        Err(err) => InvokeResult::Ok(
            json!({"marker": "state.nocap", "status": "error", "error": {"code": err.code, "message": err.message}}).to_string(),
        ),
    }
}

// NOTE: the old component_entrypoint! macro (0.5.0 ABI glue) was removed
// from greentic-interfaces-guest >=1.1. WASM export glue for these test
// fixtures will be revisited when the integration harness migrates to the
// 0.6.0 invoke contract.
