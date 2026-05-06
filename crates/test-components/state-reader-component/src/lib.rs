#![allow(dead_code)]

use greentic_interfaces_guest::state_store::{HostError, TenantCtx, read};
use serde_json::{Map, Value, json};

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
        "id": "conformance.state_reader",
        "version": "0.1.0",
        "world": "greentic:component/component@0.5.0",
        "supports": ["job"],
        "profiles": {"default": "default", "supported": ["default"]},
        "capabilities": {
            "wasi": {},
            "host": {"state": {"read": true}}
        },
        "operations": [
            {"name": "read", "input_schema": {}, "output_schema": {}}
        ]
    })
    .to_string()
}

fn invoke(op: String, input: String) -> InvokeResult {
    if op.as_str() != "read" {
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
    let ctx = tenant_ctx(&payload);
    let mut output = match payload.as_object() {
        Some(map) => map.clone(),
        None => Map::new(),
    };
    output.insert(
        "marker".to_string(),
        Value::String("state.read".to_string()),
    );
    output.insert("key".to_string(), Value::String(key.clone()));

    match read(&key, ctx.as_ref()) {
        Ok(bytes) => {
            let value = serde_json::from_slice::<Value>(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()));
            output.insert("status".to_string(), Value::String("ok".to_string()));
            output.insert("value".to_string(), value);
        }
        Err(err) => {
            output.insert("status".to_string(), Value::String("missing".to_string()));
            output.insert(
                "error".to_string(),
                json!({"code": err.code, "message": err.message}),
            );
        }
    }

    InvokeResult::Ok(Value::Object(output).to_string())
}

fn tenant_ctx(payload: &Value) -> Option<TenantCtx> {
    let tenant = payload.get("tenant")?.as_str()?;
    Some(TenantCtx {
        env: "test".to_string(),
        tenant: tenant.to_string(),
        tenant_id: tenant.to_string(),
        i18n_id: None,
        team: None,
        team_id: None,
        user: None,
        user_id: None,
        trace_id: None,
        correlation_id: None,
        attributes: Vec::new(),
        session_id: None,
        flow_id: None,
        node_id: None,
        provider_id: None,
        deadline_ms: None,
        attempt: 1,
        idempotency_key: None,
        impersonation: None,
    })
}

#[allow(dead_code)]
fn node_error(code: &str, err: HostError) -> NodeError {
    NodeError {
        code: code.to_string(),
        message: format!("{}: {}", err.code, err.message),
        retryable: false,
        backoff_ms: None,
        details: None,
    }
}

// NOTE: the old component_entrypoint! macro (0.5.0 ABI glue) was removed
// from greentic-interfaces-guest >=1.1. WASM export glue for these test
// fixtures will be revisited when the integration harness migrates to the
// 0.6.0 invoke contract.
