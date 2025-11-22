use std::sync::{Arc, Mutex};

use deploy_plan_component::{DeployPlanComponent, PlanRuntime};

#[derive(Clone)]
struct MockRuntime {
    plan_result: PlanResult,
    statuses: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
enum PlanResult {
    Ok(String),
    Err(String),
}

impl MockRuntime {
    fn ok(plan: &str) -> Self {
        Self {
            plan_result: PlanResult::Ok(plan.to_string()),
            statuses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn err(message: &str) -> Self {
        Self {
            plan_result: PlanResult::Err(message.to_string()),
            statuses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn statuses(&self) -> Vec<String> {
        self.statuses.lock().unwrap().clone()
    }
}

impl PlanRuntime for MockRuntime {
    fn emit_status(&self, message: String) {
        self.statuses.lock().unwrap().push(message);
    }

    fn get_deployment_plan(&self) -> Result<String, String> {
        match &self.plan_result {
            PlanResult::Ok(plan) => Ok(plan.clone()),
            PlanResult::Err(err) => Err(err.clone()),
        }
    }
}

#[test]
fn writes_pretty_plan_and_tracks_status() {
    let runtime = MockRuntime::ok(r#"{"key":"value","list":[1,2]}"#);
    let dir = tempfile::tempdir().expect("temp dir");

    DeployPlanComponent::run_with_runtime(&runtime, dir.path()).expect("run succeeds");

    let output = std::fs::read_to_string(dir.path().join("plan.json")).expect("plan file");
    let expected =
        serde_json::to_string_pretty(&serde_json::json!({"key":"value","list":[1,2]})).unwrap();
    assert_eq!(output, expected);

    let statuses = runtime.statuses();
    assert_eq!(
        statuses,
        vec![
            "deploy-plan-component: fetching deployment plan".to_string(),
            format!(
                "deploy-plan-component: writing {}",
                dir.path().join("plan.json").display()
            ),
            "deploy-plan-component: done".to_string()
        ]
    );
}

#[test]
fn falls_back_to_raw_plan_when_not_json() {
    let runtime = MockRuntime::ok("not-json");
    let dir = tempfile::tempdir().expect("temp dir");

    DeployPlanComponent::run_with_runtime(&runtime, dir.path()).expect("run succeeds");

    let output = std::fs::read_to_string(dir.path().join("plan.json")).expect("plan file");
    assert_eq!(output, "not-json");

    let statuses = runtime.statuses();
    assert_eq!(statuses.len(), 3, "expected fetch/write/done statuses");
}

#[test]
fn surfaces_binding_error_without_writing() {
    let runtime = MockRuntime::err("binding failed");
    let dir = tempfile::tempdir().expect("temp dir");

    let err =
        DeployPlanComponent::run_with_runtime(&runtime, dir.path()).expect_err("run should fail");
    assert!(
        err.contains("binding failed"),
        "expected binding error message, got {err}"
    );
    assert!(
        !dir.path().join("plan.json").exists(),
        "plan file should not be created on error"
    );

    let statuses = runtime.statuses();
    assert_eq!(
        statuses,
        vec!["deploy-plan-component: fetching deployment plan".to_string()],
        "status log should stop after fetch failure"
    );
}
