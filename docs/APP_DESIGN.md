# Greentic Integration App – Design Notes

## Goals
- Provide a single binary (`greentic-integration`) that glues the runner, pack
  registry, messaging adapters, and test harness hooks together.
- Treat the binary as the local/dev entrypoint so `make dev.min` and the smoke
  harness can call it directly (instead of reaching into workspace internals).
- Keep configuration declarative: allow TOML/YAML + env overrides so CI and
  `compose/stack.yml` share the same contract.

## Command Surface
```
greentic-integration serve --config config/dev.toml
greentic-integration packs validate --packs packs/*
greentic-integration packs list --tenant acme --team ops --user user-123
greentic-integration sessions purge --tenant acme --user user-123
```

### `serve`
Runs the long-lived process that hosts the HTTP/WebSocket ingress and proxies
traffic to the Greentic runner. Responsibilities:
1. Boot telemetry/logging (JSON logs, structured spans).
2. Load packs (tenant defaults plus optional team/user overrides).
3. Wire channel adapters (webchat Direct Line stub, simulated providers, etc).
4. Own session + state stores (in-memory by default, Redis when configured).
5. Expose health/session endpoints used by `make stack-up`, CI probes, and
   local debugging (see “HTTP Surface” below).

Arguments / flags:
- `--config <path>` (default `config/dev.toml`)
- `--watch` to enable pack auto-reload for local dev

### `packs validate`
Convenience wrapper around the existing `scripts/packs_test.py`. It keeps the
validation logic close to the CLI so developers do not need parallel tooling.

### `packs list`
Prints the pack ID/name/path discovered under `[packs].root`. Accepts optional
`tenant`, `team`, and `user` flags to mirror the runner lookup hierarchy (from
tenant:team:user down to tenant). The command also prints which override keys
resolved (and which were missing) so you can debug fallback behavior.

### `packs reload`
`greentic-integration packs reload --server http://localhost:8080` POSTs to
`/packs/reload` so a running server refreshes its pack index immediately. When
`--server` is omitted, the command performs a local rebuild for inspection and
prints the resulting packs (useful before triggering the real server reload).

### `sessions purge`
Used by end-to-end tests to guarantee a clean slate. Accepts tenant/team/user
filters and deletes matching sessions from the configured store.

### `sessions resume`
`greentic-integration sessions resume --user user-123 --payload '{"text":"hi"}'`
POSTs to `/sessions/resume`, which finds the matching session, echoes a runner
event, and clears the stored resume point. Optional `--tenant/--team` override
defaults, and `--server` changes the target host (defaults to
`http://localhost:8080`).

### `sessions list`
Lists resumable sessions via `/sessions` with the same tenant/team/user filters.

### `runner emit`
Submits (or clears) synthetic activity data through the runner proxy. Accepts
`--flow`, `--tenant`, `--team`, `--user`, and optional JSON `--payload`. Add
`--server URL` to hit `/runner/emit`; combine with `runner events` /
`runner clear` to inspect or reset the log remotely.

## Configuration Layout
```toml
[server]
listen_addr = "0.0.0.0:8080"

[packs]
root = "packs"
default = "acme"

[runner]
wasm_cache = ".cache/wasm"

[stores.session]
backend = "memory" # or "redis"
redis_url = "redis://localhost:6379/3"

[stores.state]
backend = "memory" # or "redis"
redis_url = "redis://localhost:6379/4"

[defaults]
tenant = "dev"
team = "team-ops"
```

Environment variables (prefixed with `GREENTIC_`) override individual values so
CI pipelines can inject secrets without touching files.

## Runtime Architecture
1. `ConfigLoader` reads the file/env overrides and produces a strongly typed
   `AppConfig`.
2. `BootstrapContext` owns shared singletons (logger guard, Arc<SessionStore>,
   Arc<StateStore>, pack resolver).
3. `serve` spins up the following async tasks:
   - HTTP ingress (axum or hyper) for `/healthz`, `/directline/*`.
   - Runner bridge task that translates inbound activities into
     `RunnerHost::handle_activity` calls.
   - Optional file watcher for pack hot reloads.
4. Each channel adapter consults the `SessionStore` before invoking the runner,
   enabling resume semantics described in the greentic-runner design.

## HTTP Surface
- `GET /healthz` – simple readiness probe consumed by compose/CI.
- `GET /packs?[tenant=...&team=...&user=...]` – dumps the pack index
  (id/name/path). When tenant/team/user are provided, the server resolves the
  most specific match (tenant:team:user → tenant:team → tenant). This mirrors
  the runner lookup order used for flow overrides.
- `POST /packs/reload` – rebuilds the pack index and notifies the runner proxy.
  Returns the same structure as `GET /packs` so callers can confirm the new
  state immediately.
- `GET /sessions?tenant=acme&team=team-ops&user=user-123` – returns
  `{"count":N,"sessions":[...]}` where each entry exposes `tenant`, `team`,
  `user`, and a nested `cursor { flow_id, node_id }` plus `updated_at_epoch_ms`
  and the raw `context` blob.
- `DELETE /sessions` – accepts filters via query string and/or JSON body
  (identical shape to GET). Responds with `{ "removed": <count> }`, allowing
  smoke tests or manual resets without shelling out to the CLI subcommand.
- `POST /sessions` – seeds or overwrites a session. If `key` is omitted, the
  server generates a UUID. `tenant`/`team` fall back to `[defaults]` when not
  provided, while `user` remains required.
- `POST /sessions/resume` – finds the session by tenant/team/user, emits a
  runner event (echo stub for now), and clears the session entry so the next
  message starts fresh.
- `GET /runner/events` – returns the cached list of synthetic runner events
  produced by `runner emit` calls (CLI or HTTP). Helpful for verifying how the
  future runner integration will log activity.
- `DELETE /runner/events` – clears the cached events (useful between test runs).
- `POST /runner/emit` – same payload as the CLI command. Stores a `RunnerEvent`
  entry, echoes the payload in `result.echo`, and simulates the runner loop.
- `make app.test` – runs the app crate’s unit tests (session store, resume flow,
  runner emit stubs) so contributors can verify changes locally.

## Implementation Phases
1. **This change**: land the CLI skeleton plus config loader so downstream work
   can depend on a concrete binary target.
2. Wire up logging + config parsing to the `serve` command.
3. Integrate real runner + session/state stores once those crates land in the
   workspace.
4. Add subcommands for packs/session automation as the harness grows.
