# Pack Fixtures

This directory tracks the canonical pack fixtures used across the Greentic integration
harness. Each pack folder contains:

- `pack.json` – Manifest with metadata, scenario entries, and golden snapshot pointers.
- `scenarios/` – Source scenario definitions consumed by greentic-dev.
- `golden/` – Renderer/output snapshots consumed by greentic-pack simulations.
- `README.md` – Human context for the scenario.

Fixtures currently bundled:

1. `demo-menu` – Landing menu buttons and happy-path branching.
2. `network-scenario-min` – Minimal network retry/reconnect flow.
3. `adaptive-basic` – Slot-filling adaptive dialog.
4. `adaptive-advanced` – Branching adaptive dialog with provider fallback.

The validation target (`make packs.test`) ensures manifests stay well-formed and that every
scenario references an existing golden snapshot. When the real `greentic-dev` and
`greentic-pack` CLIs are available locally, export `GREENTIC_PACK_VALIDATE=1` to opt-in to
their execution within the validation script.
