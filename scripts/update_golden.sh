#!/usr/bin/env bash
set -euo pipefail

if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
  echo "[error] Working tree is dirty. Commit or stash changes before updating golden snapshots." >&2
  exit 1
fi

if [[ ${UPDATE_GOLDEN:-0} != 1 ]]; then
  echo "[error] Set UPDATE_GOLDEN=1 to regenerate golden snapshots." >&2
  exit 1
fi

LOG_DIR=".logs"
LOG_FILE="${LOG_DIR}/golden-update.log"
mkdir -p "${LOG_DIR}"
: >"${LOG_FILE}"

set -x
make render.snapshot | tee -a "${LOG_FILE}"
set +x

cat <<'MSG'
Golden update completed. Review git diff and commit the changes if they look correct.
MSG
