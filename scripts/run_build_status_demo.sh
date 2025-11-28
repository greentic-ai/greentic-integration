#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLOW="${FLOW:-$ROOT/flows/events_to_message/build_status_notifications.ygtc}"
CONFIG="${CONFIG:-$ROOT/configs/demo_local.yaml}"
RUNNER_BIN="${RUNNER_BIN:-greentic-runner}"
PAYLOAD="${PAYLOAD:-$ROOT/samples/payloads/build_status_event.json}"

if ! command -v "$RUNNER_BIN" >/dev/null 2>&1; then
  echo "Runner binary '$RUNNER_BIN' not found on PATH." >&2
  exit 1
fi

echo "Running build status notification demo..."
echo "Flow:    $FLOW"
echo "Config:  $CONFIG"
echo "Payload: $PAYLOAD (set SHOW_PAYLOAD=1 to print)"
echo

"${SHOW_PAYLOAD:-false}" && cat "$PAYLOAD" && echo

"$RUNNER_BIN" run-flow \
  --flow-file "$FLOW" \
  --config "$CONFIG"
