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

TOOLCHAIN="${RUST_TOOLCHAIN:-stable}"
if ! rustup toolchain list | grep -q "${TOOLCHAIN}"; then
  log "error: rustup toolchain '${TOOLCHAIN}' not installed. Run 'rustup toolchain install ${TOOLCHAIN} --profile minimal --component clippy --component rustfmt'."
  exit 1
fi

CARGO_CMD=(cargo "+${TOOLCHAIN}")

run_step "cargo fmt" "${CARGO_CMD[@]}" fmt -- --check
run_step "cargo clippy" "${CARGO_CMD[@]}" clippy --all-targets --all-features -- -D warnings
run_step "cargo test" "${CARGO_CMD[@]}" test --workspace
run_step "make packs.test" make packs.test
run_step "make render.snapshot" make render.snapshot
run_step "make runner.smoke" make runner.smoke
run_step "make webchat.contract" make webchat.contract
run_step "make webchat.e2e" make webchat.e2e

log "✅ Local checks completed successfully."
