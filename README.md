# Greentic Integration

This repository hosts the integration harness for the Greentic demo stack. It provides
infrastructure scaffolding, golden fixtures, and automated test targets that exercise the
full local environment (runner, packs, providers, and WebChat UI).

## Project Roadmap

The integration effort follows the PR-INT series (PR-INT-01 through PR-INT-15). Each step
adds a focused capability—from bootstrapping the repository, to provider simulators,
Playwright end-to-end tests, and documentation for adding new scenarios. Refer to
`docs/` for the detailed implementation plan.

## Getting Started

1. Install the required tooling (Docker, Rust, Node.js 18+, and Make).
2. Clone this repository.
3. Run `make help` to list the available developer commands.

## Repository Layout

```
compose/    Local infrastructure definitions (Docker Compose stacks)
harness/    Rust crates/binaries for simulators and smoke tests
packs/      Demo pack fixtures and golden snapshots
scripts/    Utility scripts (golden updates, repo pinning, etc.)
docs/       Project documentation and onboarding guides
```

> **Note:** Most directories are scaffolds as of PR-INT-01. Subsequent PR-INT tasks will
> populate them with runnable code, fixtures, and tests.

## Local Infrastructure Stack

Run `make stack-up` to start the deterministic local dependencies defined under
`compose/stack.yml`. The stack currently includes:

- NATS with JetStream + monitoring endpoint (ports 4222/8222)
- Redis 7 (port 6379)
- An ingress stub (port 8080) that surfaces a `/healthz` probe

You can verify the ingress health check with:

```bash
curl -fsS http://localhost:8080/healthz
```

Stop and clean up the containers with `make stack-down`.

## Pack Fixtures

Pack definitions live under `packs/`. Each pack exposes a manifest (`pack.json`), scenario
definitions, and golden transcripts. Run `make packs.test` to ensure manifests stay
well-formed. Set `GREENTIC_PACK_VALIDATE=1` to opt-in to the real `greentic-dev` /
`greentic-pack` CLI checks once those binaries are available locally.

## Renderer Snapshots

Provider simulators live under `harness/providers-sim`. Run `make render.snapshot` to execute
the snapshot test suite, which compares renderer metrics against
`harness/providers-sim/golden/render_reports.json`. When intentionally updating packs or the
renderer logic, refresh the golden file via:

```bash
UPDATE_GOLDEN=1 make render.snapshot
```

## Runner Smoke Harness

`make runner.smoke` executes the deterministic runner harness housed in
`harness/runner-smoke`. It replays canned dev-mode traces to verify session continuity,
tenant isolation, state write expectations, and once-only effect log semantics before hooking
into the real Runner binary. The effect log contract lives in
`harness/runner-smoke/effect_log.schema.json`.

## Demo Payload Replays

- `make demo.replay.build` / `make demo.replay.chat` replay the sample EventEnvelope/ChannelMessageEnvelope payloads through the runner emit proxy.
- CI starts the `greentic-integration` server and runs these targets with `USE_SERVER=1` so they POST to `http://localhost:8080/runner/emit`. Locally, the make targets default to the in-process stub unless you set `USE_SERVER=1`.

## Dev Mode Check

`make dev.min` now runs `scripts/dev-check/check.sh`, which verifies essential environment
variables (`DEV_API_KEY`, `DEV_TENANT_ID`), ensures telemetry is disabled for local runs,
checks Docker/Docker Compose availability, and looks for the hot-reload token under
`.dev/reload.token`. Logs land in `.logs/dev-check.log`.

## WebChat Contract Tests

`make webchat.contract` hits the Direct Line-compatible backend endpoints
(`/tokens/generate`, `/conversations`, `/activities`). By default it runs against an
in-process stub server so the suite works offline. Point it at a real backend by exporting
`WEBCHAT_BASE_URL=https://your-service.example.com`.

## WebChat Playwright E2E

`make webchat.e2e` executes the Playwright UI suite located under `webchat-e2e/`. Install the
Node dependencies (`npm install`) when network access is available. The harness automatically
tries to download the browser binaries locally (`PLAYWRIGHT_BROWSERS_PATH=0`); if that fails,
the target will log a warning and skip execution. Review `.logs/webchat-e2e.log` for details.
The default run targets Chromium; set `PLAYWRIGHT_PROJECT=firefox` (or any other configured
project) to run a different browser locally.

## Golden Snapshot Management

Golden reports (renderer outputs, etc.) should only change when intentionally refreshed. Run:

```bash
UPDATE_GOLDEN=1 make golden.update
```

The script enforces a clean working tree, regenerates snapshots (currently via
`make render.snapshot`), and writes logs to `.logs/golden-update.log`. Commit the resulting
changes to keep CI green; any drift detected by CI indicates the golden refresh step was
skipped.

## Continuous Integration

`.github/workflows/integration.yml` runs on pushes/PRs and nightly at 05:00 UTC. It fans out
into four jobs (lint, packs, harness, and webchat). The webchat job runs Chromium in the fast
path and expands to Chromium + Firefox on the nightly schedule. Cargo and npm/Playwright
artifacts are cached to keep the workflow fast.

## Local CI

Run `./ci/local_check.sh` before pushing to ensure the same suite passes locally (fmt, clippy,
workspace tests, and all Make targets including packs, harnesses, and WebChat checks).

## Cross-Repo Pinning

Use `./scripts/pin_repo.sh <org/repo> <sha>` to write a `[patch]` override into
`.cargo/config.toml`. This lets you test unreleased dependencies (e.g.
`./scripts/pin_repo.sh greentic-ai/greentic-messaging deadbeef`). Remove the generated section
from the config (or run the script again with a new SHA) to unpin.

## Contributor Docs

- `docs/ADDING_A_SCENARIO.md` – walkthrough for creating new packs/scenarios and refreshing
  golden data.
- `docs/ADDING_A_PROVIDER_SIM.md` – process for extending provider simulators and updating
  capability parity checks.

## End-to-End Harness

The E2E harness lives in `crates/app/src/harness`. Run the smoke test with:

```bash
cargo test -p greentic-integration e2e_smoke
```

`TestEnv` writes logs/artifacts under `target/e2e/<test-name>/`; set `E2E_TEST_NAME` to control
the folder name (defaults to a sanitized thread name or timestamp).

Infra-backed E2E (NATS + Postgres) uses Docker Compose in `tests/compose/compose.e2e.yml`:

```bash
cargo test -p greentic-integration e2e_infra
```

Logs are captured under `target/e2e/<test-name>/logs/compose.log` before teardown.

Pack lifecycle and scenario DSL tests:

```bash
cargo test -p greentic-integration e2e_pack_lifecycle
cargo test -p greentic-integration e2e_scenario_smoke
cargo test -p greentic-integration e2e_multi_tenant_isolation
```

Pack helpers look for binaries under `tests/bin/`, `target/{release,debug}/`, or PATH and stub when unavailable, writing artifacts to `target/e2e/<test>/artifacts/`.

Greentic stack boot (runner/deployer/store) uses locally available binaries (looked up under
`tests/bin/`, `target/{release,debug}/`, or PATH). The stack test will skip if binaries are
missing:

```bash
cargo test -p greentic-integration e2e_stack_boot
```

## E2E Test Tiers (CI)

- **L0/L1 (PR)**: `e2e_smoke`, `e2e_scenario_smoke`, `e2e_retry_backoff_flaky_tool`, `e2e_config_precedence`, `e2e_pack_lifecycle`
- **L2 (nightly/dispatch)**: `e2e_infra`, `e2e_stack_boot`, `e2e_multi_tenant_isolation` plus L0/L1 set

On CI failure, `target/e2e/**` is uploaded for debugging (logs, observations, artifacts).

## Local E2E Runner

Use `./scripts/e2e.sh <tier>` for local runs:

```bash
./scripts/e2e.sh l1          # run L1 suite
./scripts/e2e.sh l2 --focus e2e_multi_tenant_isolation
```

Flags:
- `--focus <pattern>` – run a single test
- `E2E_KEEP=1` – retain `target/e2e` between runs
- `RUST_LOG=info` (default) can be overridden for verbose logs

On failure, the script prints the paths under `target/e2e` for quick inspection.
