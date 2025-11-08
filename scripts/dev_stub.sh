#!/usr/bin/env bash
set -euo pipefail

target="${1:-}"
shift || true
message="${*:-This is a stub target. Track the relevant PR-INT milestone to replace it with real logic.}"

if [[ -z "${target}" ]]; then
  echo "Usage: $0 <target> [message]" >&2
  exit 2
fi

log_dir=".logs"
mkdir -p "${log_dir}"
log_file="${log_dir}/${target}.log"
timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

{
  echo "[${timestamp}] ${target}"
  echo "${message}"
} | tee "${log_file}" >/dev/null

echo "Stub target '${target}' completed. Log written to ${log_file}."
