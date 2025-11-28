#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FLOW="${FLOW:-$ROOT/flows/chat_driven/repo_assistant.ygtc}"
CONFIG="${CONFIG:-$ROOT/configs/demo_local.yaml}"
RUNNER_BIN="${RUNNER_BIN:-greentic-runner}"
PAYLOAD="${PAYLOAD:-$ROOT/samples/payloads/channel_message.json}"
REBUILD_PAYLOAD="${REBUILD_PAYLOAD:-$ROOT/samples/payloads/rebuild_request_event.json}"

if ! command -v "$RUNNER_BIN" >/dev/null 2>&1; then
  echo "Runner binary '$RUNNER_BIN' not found on PATH." >&2
  exit 1
fi

echo "Running chat-driven Repo Assistant demo..."
echo "Flow:    $FLOW"
echo "Config:  $CONFIG"
echo "Payload: $PAYLOAD (set SHOW_PAYLOAD=1 to print)"
echo "Rebuild payload (optional branch): $REBUILD_PAYLOAD"
echo

"${SHOW_PAYLOAD:-false}" && cat "$PAYLOAD" && echo

"$RUNNER_BIN" run-flow \
  --flow-file "$FLOW" \
  --config "$CONFIG"
