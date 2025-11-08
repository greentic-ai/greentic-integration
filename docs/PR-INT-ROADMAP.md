# Greentic Integration – PR-INT Roadmap

This document tracks the phased implementation plan for the integration repository. Each
PR-INT item represents a discrete deliverable with clear acceptance criteria and a primary
command used for verification.

| ID        | Goal (abridged)                                | Acceptance (command)                          |
|-----------|------------------------------------------------|-----------------------------------------------|
| PR-INT-01 | Repo skeleton, Makefile, CI scaffold           | `make help`                                   |
| PR-INT-02 | Dev Make targets (stack, packs, runner, web)   | `make dev.min`                                |
| PR-INT-03 | Docker Compose stack (NATS, Redis, ingress)    | `make stack-up && curl -fsS :8080/healthz`    |
| PR-INT-04 | Pack fixtures + golden snapshots               | `make packs.test`                             |
| PR-INT-05 | Provider simulators + renderer snapshots       | `make render.snapshot`                        |
| PR-INT-06 | Runner smoke harness                           | `make runner.smoke`                           |
| PR-INT-07 | WebChat contract tests                         | `make webchat.contract`                       |
| PR-INT-08 | Playwright WebChat E2E                         | `make webchat.e2e`                            |
| PR-INT-09 | CI workflow (fast + nightly matrix)            | GitHub Actions                                |
| PR-INT-10 | Golden snapshot management/update command      | `make golden.update`                          |
| PR-INT-11 | Provider capability parity tests               | `cargo test -p providers-sim`                 |
| PR-INT-12 | Effect log schema + idempotency semantics      | `make runner.smoke`                           |
| PR-INT-13 | Dev-mode bootstrap verifier                    | `make dev.min` (includes dev-check)           |
| PR-INT-14 | Cross-repo pinning helpers                     | `./scripts/pin_repo.sh org/repo <sha>`        |
| PR-INT-15 | Docs for adding scenarios/provider simulators  | `docs/ADDING_*.md` (manual review)            |

The remaining sections will be populated as each phase lands.
