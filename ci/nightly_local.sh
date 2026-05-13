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

mkdir -p "${ARTIFACTS_DIR}"
mkdir -p "${PACKS_DIR}"

run_step "sync packs to dist/packs" rsync -a "${ROOT_DIR}/packs/" "${PACKS_DIR}/"

run_step "pack doctor" sh -c '
  set -eu
  : > "$1/pack_doctor.log"
  for pack_dir in "$2"/*/; do
    if [ -f "${pack_dir}pack.yaml" ]; then
      {
        printf "=== Pack doctor: %s ===\n" "${pack_dir}"
        greentic-pack doctor --validate "${pack_dir}"
      } 2>&1 | tee -a "$1/pack_doctor.log"
    fi
  done
' _ "${ARTIFACTS_DIR}" "${PACKS_DIR}"

shopt -s nullglob

for domain in messaging events secrets; do
  case "${domain}" in
    messaging)
      packs=( "${PACKS_DIR}"/messaging-*.gtpack )
      if [ ${#packs[@]} -eq 0 ]; then
        log "skipping messaging conformance: no messaging-*.gtpack in ${PACKS_DIR}"
      else
        run_step "messaging conformance (dry-run)" \
          greentic-messaging-test e2e --packs "${PACKS_DIR}" --report "${ARTIFACTS_DIR}/messaging.json" --dry-run
      fi
      ;;
    events)
      packs=( "${PACKS_DIR}"/events-*.gtpack )
      if [ ${#packs[@]} -eq 0 ]; then
        log "skipping events conformance: no events-*.gtpack in ${PACKS_DIR}"
      else
        run_step "events conformance (dry-run)" \
          greentic-events-test e2e --packs "${PACKS_DIR}" --report "${ARTIFACTS_DIR}/events.json" --dry-run
      fi
      ;;
    secrets)
      packs=( "${PACKS_DIR}"/secrets-*.gtpack )
      if [ ${#packs[@]} -eq 0 ]; then
        log "skipping secrets conformance: no secrets-*.gtpack in ${PACKS_DIR}"
      else
        run_step "secrets conformance (dry-run)" \
          greentic-secrets-test e2e --packs "${PACKS_DIR}" --report "${ARTIFACTS_DIR}/secrets.json" --dry-run
      fi
      ;;
  esac
done

packs=( "${PACKS_DIR}"/*.gtpack )
if [ ${#packs[@]} -eq 0 ]; then
  log "skipping runner conformance: no *.gtpack in ${PACKS_DIR}"
else
  run_step "runner conformance" \
    greentic-runner conformance --packs "${PACKS_DIR}" --level l2 --report "${ARTIFACTS_DIR}/runner.json"
  run_step "runner conformance (faults)" \
    greentic-runner conformance --packs "${PACKS_DIR}" --faults tests/fixtures/faults/basic.json --report "${ARTIFACTS_DIR}/faults.json"
fi

if [[ -d "${ROOT_DIR}/target/e2e" ]]; then
  run_step "copy gtest artifacts" rsync -a "${ROOT_DIR}/target/e2e/" "${ARTIFACTS_DIR}/e2e/"
fi

log "✅ Nightly local run complete."
