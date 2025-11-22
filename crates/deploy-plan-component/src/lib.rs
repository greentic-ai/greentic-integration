#![allow(dead_code)]
#![allow(unused_imports)]

use std::path::Path;

/// Minimal deploy-plan component: reads the plan via bindings and writes it to /iac/plan.json.
pub struct DeployPlanComponent;

/// Abstraction over the deploy-plan host bindings so tests can inject a mock runtime.
pub trait PlanRuntime {
    fn emit_status(&self, message: String);
    fn get_deployment_plan(&self) -> Result<String, String>;
}

/// Placeholder runtime: real bindings are expected to be provided by the host environment.
#[derive(Debug, Default)]
pub struct GuestPlanRuntime;

impl PlanRuntime for GuestPlanRuntime {
    fn emit_status(&self, _message: String) {}

    fn get_deployment_plan(&self) -> Result<String, String> {
        Err("deploy-plan bindings are provided by the host; no local WIT present".into())
    }
}

impl DeployPlanComponent {
    pub fn run() -> Result<(), String> {
        Self::run_with_runtime(&GuestPlanRuntime, Path::new("/iac"))
    }

    /// Runs the component using the provided bindings runtime, writing the plan under `output_root`.
    /// This keeps the production path at `/iac` while letting tests inject a temp directory.
    pub fn run_with_runtime(runtime: &impl PlanRuntime, output_root: &Path) -> Result<(), String> {
        runtime.emit_status("deploy-plan-component: fetching deployment plan".into());
        let plan = runtime.get_deployment_plan()?;

        let pretty_plan = serde_json::from_str::<serde_json::Value>(&plan)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or_else(|_| plan.clone());

        let output_path = output_root.join("plan.json");

        runtime.emit_status(format!(
            "deploy-plan-component: writing {}",
            output_path.display()
        ));

        std::fs::create_dir_all(output_root).map_err(|e| e.to_string())?;
        std::fs::write(&output_path, pretty_plan).map_err(|e| e.to_string())?;

        runtime.emit_status("deploy-plan-component: done".into());
        Ok(())
    }
}
