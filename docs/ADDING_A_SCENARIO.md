# Adding a Scenario

This guide explains how to add a new scenario/pack so it participates in packs.test, renderer
snapshots, runner smoke checks, and higher-level tests.

## 1. Create the Pack Skeleton

1. Under `packs/`, create a folder named after your scenario (e.g. `packs/adaptive-pro`).
2. Add:
   - `pack.json` – manifest describing metadata and scenario entries.
   - `scenarios/<scenario_id>.json` – source definition consumed by greentic-dev.
   - `golden/<scenario_id>.json` – transcript used by render snapshots.
   - `README.md` – short human context.

Example `pack.json` snippet:

```json
{
  "id": "adaptive-pro",
  "name": "Adaptive Dialog Pro",
  "version": "0.1.0",
  "description": "Captures advanced adaptive features.",
  "type": "adaptive",
  "scenarios": [
    {
      "id": "adaptive_pro",
      "entry": "scenarios/adaptive_pro.json",
      "golden": "golden/adaptive_pro.json",
      "tags": ["adaptive", "pro"]
    }
  ]
}
```

## 2. Write Scenario + Golden Files

- `scenarios/*.json` must include `scenario` (matching the manifest id), `description`, and a
  non-empty `steps` list containing `{ actor, message }` entries.
- `golden/*.json` must include `scenario_id` (matching the manifest id) and `transcript` (array
  of `"BOT/User/System: ..."` strings).

## 3. Validate via packs.test

```bash
make packs.test
```

This runs `scripts/packs_test.py`, which enforces JSON schema, ensures scenario/golden files
exist, and optionally shells out to `greentic-dev`/`greentic-pack` when
`GREENTIC_PACK_VALIDATE=1`.

## 4. Refresh Renderer Snapshots

If your scenario changes transcript output, regenerate the snapshot:

```bash
UPDATE_GOLDEN=1 make golden.update
```

Review the diff in `harness/providers-sim/golden/render_reports.json` and commit it.

## 5. Run Local CI

Finish with:

```bash
./ci/local_check.sh
```

This ensures the new pack flows through renderer snapshots, runner smoke, and WebChat
contract/e2e tests.
