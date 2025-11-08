# runner-smoke

Deterministic smoke harness that asserts runner invariants (session continuity, tenant
isolation, and state writes) using canned dev-mode traces in `cases/`. The harness ensures we
catch regressions before wiring it to the real Runner process.

Run it via `make runner.smoke`, or directly with `cargo run -p runner-smoke -- --cases <dir>`
when pointing at alternative trace folders.

## Effect Log Schema

`effect_log.schema.json` describes the once-only effect log contract (trace IDs, sequence, and
event types). The harness ensures every `state_write` event carries a unique `trace_id`; any
duplicate trace detected across sessions fails the smoke suite to guard idempotent state
writes.
