# Dev Check

`scripts/dev-check/check.sh` enforces local developer prerequisites:

- `DEV_API_KEY` – authenticates requests for local runner/dev services.
- `DEV_TENANT_ID` – associates scenarios with a tenant.
- `DEV_TELEMETRY_ENABLED` – should be `false` when running locally.
- `.dev/reload.token` – optional file indicating hot reload secrets are installed.
- Docker/Docker Compose – required for `make stack-up` and other infra commands.

Run `make dev.min` to execute the check along with other bootstrap steps. Logs are written to
`.logs/dev-check.log` for inspection.
