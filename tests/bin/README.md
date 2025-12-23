# Test Binaries

This folder is the first lookup location for integration test binaries (e.g. `greentic-runner`, `greentic-deployer`, `greentic-store`).

- **Purpose:** ensure deterministic CI runs without depending on developer PATH.
- **Layout:** place platform-specific binaries here, e.g. `tests/bin/greentic-runner` for Linux x86_64 CI.
- **Provenance:** document how each binary was built/pinned and include checksums alongside the binaries.
- **Strict modes:** when `GREENTIC_STACK_STRICT=1` or `GREENTIC_INTEGRATION_STRICT=1`, missing binaries here will fail the tests (no fallback).
- **CI download:** `.github/workflows/e2e.yml` calls `scripts/fetch_greentic_binaries.sh` to download pinned release artifacts into `tests/bin/linux-x86_64/` and verify SHA256 checksums. Set the following env vars in CI (no defaults):  
  - `GREENTIC_RUNNER_URL`, `GREENTIC_RUNNER_SHA256`  
  - `GREENTIC_DEPLOYER_URL`, `GREENTIC_DEPLOYER_SHA256`  
  - `GREENTIC_STORE_URL`, `GREENTIC_STORE_SHA256`  
  - Optional: `GREENTIC_RUNNER_VERSION`, `GREENTIC_DEPLOYER_VERSION`, `GREENTIC_STORE_VERSION` (metadata only)

This repo expects CI to supply these binaries via the download step; local developers can drop compatible binaries here if desired.
