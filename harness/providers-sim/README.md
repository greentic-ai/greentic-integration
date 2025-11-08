# providers-sim

Rust crate that houses deterministic provider simulators for the Greentic integration
harness. The current focus is the renderer snapshot flow exposed through
`make render.snapshot`, which validates renderer output metrics against
`golden/render_reports.json`.

Use `UPDATE_GOLDEN=1 make render.snapshot` when intentionally updating pack transcripts or
expected renderer behavior.

## Capabilities Parity

Provider feature parity is declared in `capabilities/providers.yaml`. Run
`cargo test -p providers-sim` to ensure the simulator implements every capability supported by
the reference provider unless an explicit downgrade rationale is documented.
