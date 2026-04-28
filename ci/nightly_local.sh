#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

log() {
  printf "\n[%s] %s\n" "$(date -u +%H:%M:%S)" "$*"
}

run_step() {
  local description=$1
  shift
  log "➡️  ${description}"
  "$@"
}

ARTIFACTS_DIR="${ROOT_DIR}/artifacts/nightly_local"
PACKS_DIR="${ROOT_DIR}/dist/packs"

ensure_command() {
  local cmd=$1
  local crate=${2:-$1}
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    run_step "install ${crate}" cargo binstall "${crate}" --no-confirm --force
  fi
}

ensure_optional_command() {
  local cmd=$1
  local crate=${2:-$1}
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    if cargo binstall "${crate}" --no-confirm --force; then
      return 0
    fi
    log "⚠️  optional tool unavailable: ${crate}"
  fi
}

run_optional_step() {
  local description=$1
  local cmd=$2
  shift 2
  if command -v "${cmd}" >/dev/null 2>&1; then
    run_step "${description}" "$cmd" "$@"
  else
    log "⚠️  skipping ${description}: ${cmd} not installed"
  fi
}

mkdir -p "${ARTIFACTS_DIR}"
mkdir -p "${PACKS_DIR}"

run_step "ensure wasm32-wasip2 target" rustup target add wasm32-wasip2
ensure_command greentic-dev
ensure_command greentic-component
ensure_command greentic-flow
ensure_command cargo-component

run_step "sync packs to dist/packs" rsync -a "${ROOT_DIR}/packs/" "${PACKS_DIR}/"

run_step "pack validation" \
  sh -c "cargo run -p greentic-integration --bin greentic-integration -- packs validate | tee \"${ARTIFACTS_DIR}/packs_validate.log\""

run_step "provider-core conformance" \
  sh -c "GREENTIC_PROVIDER_CORE_ONLY=1 cargo test -p greentic-integration --test pr14_provider_core_e2e -- --nocapture | tee \"${ARTIFACTS_DIR}/provider_core_conformance.log\""

run_step "component contract tests" \
  sh -c "cargo test -p greentic-component --all-features | tee \"${ARTIFACTS_DIR}/component_tests.log\""

run_step "gtest smoke with triage" \
  cargo run -p greentic-integration-tester -- \
    --test tests/gtests/00_smoke_validator.gtest \
    --triage-flakes \
    --triage-runs 3
run_step "gtest capture with triage" \
  cargo run -p greentic-integration-tester -- \
    --test tests/gtests/01_capture_and_expect.gtest \
    --triage-flakes \
    --triage-runs 3

if [[ -d "${ROOT_DIR}/target/e2e" ]]; then
  run_step "copy gtest artifacts" rsync -a "${ROOT_DIR}/target/e2e/" "${ARTIFACTS_DIR}/e2e/"
fi

if [[ -d "${ROOT_DIR}/target/flake-artifacts" ]]; then
  run_step "copy flake artifacts" rsync -a "${ROOT_DIR}/target/flake-artifacts/" "${ARTIFACTS_DIR}/flake-artifacts/"
fi

log "✅ Nightly local run complete."
