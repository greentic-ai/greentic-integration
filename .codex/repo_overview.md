# Repository Overview

## 1. High-Level Purpose
- Integration harness for the Greentic demo stack, combining Rust crates, pack fixtures, and helper scripts to exercise runner/providor flows end-to-end.
- Provides a lightweight HTTP/CLI app for managing packs and sessions, simulator crates for renderer and runner invariants, deterministic demo flows/payloads, and utility tooling for local infra, validation, and Playwright WebChat checks.

## 2. Main Components and Functionality
- **Path:** `crates/app` — Rust CLI/HTTP service (`greentic-integration`) that loads config defaults, indexes packs, and offers endpoints for health, pack list/reload, runner event emit/list/clear, and session upsert/list/resume/purge; includes CLI helpers for pack validation (`scripts/packs_test.py`), listing, deployment plan inference, runner emit/events/clear, and session maintenance; session store supports in-memory or file-backed JSON; runner proxy synthesizes/records stub events; E2E harness (`harness` module) provisions per-test dirs, Docker Compose NATS/Postgres stack, optional Greentic stack process boot with log capture, pack build/verify/install helpers, fixtures loader/normalizer, config/secret precedence utilities, scenario DSL for NATS-driven flows, and tenant-scoped helpers.
- **Path:** `crates/deploy-plan-component` — WASM-friendly library that writes a deployment plan JSON exposed by host bindings to `/iac/plan.json`, with tests using an injectable runtime to verify status logging and pretty-printing behavior.
- **Path:** `harness/providers-sim` — Renderer/provider simulator that loads pack manifests and golden transcripts to produce deterministic `RenderReport` records and validates scenario/manifest consistency; includes capability parity checks against `capabilities/providers.yaml` and golden snapshot comparisons in `golden/render_reports.json`.
- **Path:** `harness/runner-smoke` — CLI harness that loads JSON cases from `harness/runner-smoke/cases` to verify runner invariants (tenant isolation, ordered sequences, state snapshot presence, once-only trace IDs) and reports aggregated results.
- **Path:** `packs/` — Canonical pack fixtures (manifests, scenarios, golden transcripts, README context) including demo, adaptive, deployment, and network examples; validated by `make packs.test` / `scripts/packs_test.py`, and consumed by renderer snapshot tests.
- **Path:** `flows/` — YAML flow definitions: `chat_driven/repo_assistant.ygtc` routes webchat ingress to a worker with optional rebuild event emission; `events_to_message/build_status_notifications.ygtc` bridges build-status events to messaging channels.
- **Path:** `scripts/` — Utility scripts: pack validation (`packs_test.py`), dev environment checks (`dev-check/check.sh`), golden snapshot refresh (`update_golden.sh`), deploy component build, demo payload replays, Playwright/WebChat runners, and stub dev bootstrap logging (`dev_stub.sh`).
- **Path:** `fixtures/` — Standardized test fixtures (`inputs/`, `expected/`, `packs/`, `config/`, `secrets/`) plus JSON/text loader helpers and normalizer in `crates/app/src/fixtures.rs`; includes `packs/hello` minimal pack project and fallback `hello.gtpack`.
- **Path:** `compose/stack.yml` — Docker Compose stack for local infra (NATS with JetStream, Redis, nginx ingress healthz probe) used by `make stack-up/down`.
- **Path:** `tests/compose/compose.e2e.yml` — Docker Compose stack for E2E harness (NATS + Postgres) driven by `TestEnv` for infra tests.
- **Path:** `configs/demo_local.yaml` — Sample runner configuration wiring local demo providers/bridges/workers to fixture components under `packs/integration-demos/components/`.
- **Path:** `webchat-e2e/` — Playwright UI test harness with config and tests runnable via `make webchat.e2e` (uses local stub backend by default).
- **Path:** `samples/payloads/` — JSON payload fixtures (build status, rebuild request, channel message) exercised by crate tests and replay scripts.
- **Path:** `scripts/e2e.sh` — Local E2E runner for tiered suites (`l0|l1|l2`) with focus filtering and artifact hints.
- **Path:** `.github/workflows/e2e.yml` — CI workflow running L0/L1 on PR and L2 on nightly/dispatch, uploading `target/e2e` artifacts on completion/failure.

## 3. Work In Progress, TODOs, and Stubs
- `crates/app/src/main.rs:541-544` — Pack watch mode flagged as “not implemented yet”; server continues without live pack reload despite `--watch`.
- `crates/app/src/main.rs:711-729` — Session store Redis backend explicitly unsupported (bails with “Redis backend not supported yet”).
- `crates/app/src/main.rs:1371-1454` — Runner proxy (`RunnerHostProxy`/`proxy_runner_loop`) only logs and echoes synthetic events; no real runner integration or host bridge.
- `crates/app/src/harness/services.rs` & `crates/app/tests/e2e_stack_boot.rs` — Stack boot relies on local Greentic binaries (runner/deployer/store) discovered under `tests/bin` or `target/{release,debug}`; test skips when binaries or Docker are absent.
- `crates/app/tests/e2e_retry_backoff.rs` — Retry/backoff test uses a local flaky tool stub; real runner/tool wiring still TODO when binaries are available.
- `crates/app/tests/e2e_multi_tenant_isolation.rs` — Tenant isolation test uses NATS subjects + tenant-scoped secrets/state under artifacts; real runner/store wiring still pending.
- `.github/workflows/e2e.yml` — E2E CI tiers assume Docker availability and may skip tests when unavailable; artifacts uploaded for debugging but rely on `target/e2e` contents.
- `crates/deploy-plan-component/src/lib.rs:15-24` — `GuestPlanRuntime` returns an error because deploy-plan host bindings are absent locally; `DeployPlanComponent::run` is effectively a placeholder until real bindings exist.
- `scripts/dev_stub.sh:4-23` — Stub target invoked by `make dev.min` / `make dev.full`, only logs a placeholder message to `.logs/<target>.log`.

## 4. Broken, Failing, or Conflicting Areas
- No failing tests observed; `cargo test --workspace` currently passes. E2E infra/stack tests are skip-aware when Docker or Greentic binaries are unavailable.

## 5. Notes for Future Work
- Implement pack watch reloading and real runner integration so `--watch` and runner proxy calls reflect live system behavior.
- Add Redis-backed session store support and wire it into config defaults where appropriate.
- Replace deploy-plan placeholder runtime with actual host bindings/WIT implementation and ensure `DeployPlanComponent::run` succeeds in production environments.
- Replace stub dev bootstrap targets with real setup flows; keep golden update and validation scripts aligned with evolving pack/renderer specs.
- Provide local Greentic binaries (runner/deployer/store) and real health endpoints so stack boot tests can fully exercise the stack without skipping.
- Wire retry/backoff, tenant isolation, and pack lifecycle tests to real runner/tool/store once available; extend CI tiers to include new coverage without excessive runtime.
