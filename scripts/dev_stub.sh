#!/usr/bin/env bash
set -euo pipefail

# Lightweight developer bootstrap: validate local tooling, Docker, and greentic-dev presence.

target="${1:-dev}"
log_dir=".logs"
mkdir -p "${log_dir}"
log_file="${log_dir}/${target}.log"

timestamp() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

log() {
  echo "[$(timestamp)] $*" | tee -a "${log_file}"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    log "ERROR: missing required command '$1'"
    exit 1
  fi
}

log "starting dev bootstrap (${target})"
log "cwd: $(pwd)"

require_cmd rustc
require_cmd cargo
require_cmd docker
require_cmd node

log "rustc: $(rustc --version)"
log "cargo: $(cargo --version)"
log "node: $(node --version)"

if docker info >/dev/null 2>&1; then
  log "docker: available"
else
  log "ERROR: docker daemon not reachable (start Docker Desktop or dockerd)"
  exit 1
fi

if command -v greentic-dev >/dev/null 2>&1; then
  log "greentic-dev: $(greentic-dev --version)"
else
  log "WARN: greentic-dev not found on PATH; install with 'cargo binstall greentic-dev'"
fi

log "env check complete. Log written to ${log_file}"
