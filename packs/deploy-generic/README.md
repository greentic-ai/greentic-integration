## Deploy Generic Pack

- **Kind:** deployment (demonstration only)
- **Purpose:** Shows a deployment flow that reads a `DeploymentPlan` (via the deploy-plan WIT
  world) and emits IaC artifacts to `/iac/plan.json`.
- **Component stub:** `components/deployment_component.yaml` advertises `host.iac` capabilities
  and the `greentic:deploy-plan@1.0.0` world; no WASM is bundled.
- **Flow:** `flows/deploy_generic_iac.ygtc` shows a deployment flow structure (events flow,
  render node → done) that would be executed by a deployment component.
- **WASM guest:** build via `make component.deploy-plan` to copy
  `deploy_plan_component.wasm` into `components/` for local runs.

### Scenario
- `deploy_plan_written` – Bot prepares a deployment plan, host exposes `/iac/plan.json`, and
  the flow reports completion. The golden transcript mirrors the expected messages.

This pack fits the generic Greentic spec: IDs and kinds are opaque, `kind` is just a hint,
and IaC semantics remain provider-neutral.
