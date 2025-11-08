# Adding a Provider Simulator

The `providers-sim` crate supplies deterministic provider behavior for renderer snapshot and
parity tests. Follow this guide to introduce a new simulator or extend capabilities.

## 1. Extend Capabilities Map

Edit `harness/providers-sim/capabilities/providers.yaml`:

- Add your provider under `providers` with a `capabilities` array.
- If the simulator intentionally lacks a capability found in the reference provider, document
the downgrade under `downgrades` with a reason.

## 2. Implement Simulator Logic

Add modules/functions under `harness/providers-sim/src/` that expose deterministic outputs for
your provider. Reuse the existing `simulate_render` pattern or introduce new APIs as needed.

## 3. Update Tests

- Add/extend unit tests under `harness/providers-sim/tests/` to cover new behavior.
- Ensure the capabilities parity test still passes: it enforces that any reference capability
  missing from the simulator is documented in the yaml file.

## 4. Regenerate Golden Outputs

If the simulator changes renderer behavior or transcripts, rerun:

```bash
UPDATE_GOLDEN=1 make golden.update
```

## 5. Run Local CI

Execute `./ci/local_check.sh` to exercise cargo fmt/clippy/tests plus the higher-level make
targets (packs, runner, webchat). Commit the yaml/code/golden diffs together.
