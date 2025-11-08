#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

LOG_DIR=".logs"
LOG_FILE="${LOG_DIR}/dev-check.log"
mkdir -p "${LOG_DIR}"
: >"${LOG_FILE}"

log() {
  local level=$1
  shift
  printf "[%s] %s\n" "${level}" "$*" | tee -a "${LOG_FILE}"
}

require_var() {
  local name=$1
  if [[ -z "${!name:-}" ]]; then
    log "error" "Environment variable ${name} is required for dev mode."
    return 1
  fi
  log "ok" "${name} is set."
}

check_reload() {
  local file="${ROOT_DIR}/.dev/reload.token"
  if [[ ! -f "${file}" ]]; then
    log "warn" "${file} missing. Hot reload may not function."
  else
    log "ok" "Reload token present."
  fi
}

check_telemetry() {
  local telemetry="${DEV_TELEMETRY_ENABLED:-true}"
  if [[ "${telemetry}" != "false" ]]; then
    log "warn" "DEV_TELEMETRY_ENABLED should be 'false' for local dev. Current: ${telemetry}"
  else
    log "ok" "Telemetry disabled for dev."
  fi
}

check_docker() {
  if command -v docker >/dev/null 2>&1; then
    if docker compose version >/dev/null 2>&1 || docker-compose version >/dev/null 2>&1; then
      log "ok" "Docker Compose available."
    else
      log "warn" "Docker CLI found but 'docker compose' is unavailable. Install Docker Desktop 2.x+ or docker-compose."
    fi
  else
    log "error" "Docker CLI not found. Install Docker to run local services."
    return 1
  fi
}

main() {
  require_var "DEV_API_KEY"
  require_var "DEV_TENANT_ID"
  check_telemetry
  check_reload
  check_docker
  log "info" "Dev-check completed."
}

main "$@"
