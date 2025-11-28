#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-greentic-integration}"
FLOW_ID="${FLOW_ID:-repo_assistant_chat}"
PAYLOAD_FILE="${PAYLOAD_FILE:-$ROOT/samples/payloads/channel_message.json}"
TENANT="${TENANT:-dev}"
USER="${USER:-user-1}"
SERVER="${SERVER:-http://localhost:8080}"
USE_SERVER="${USE_SERVER:-0}"

if ! command -v "$BIN" >/dev/null 2>&1; then
  echo "Binary '$BIN' not found on PATH. Build greentic-integration or set BIN=/path/to/bin." >&2
  exit 1
fi

if [[ ! -f "$PAYLOAD_FILE" ]]; then
  echo "Payload file not found: $PAYLOAD_FILE" >&2
  exit 1
fi

PAYLOAD_JSON="$(cat "$PAYLOAD_FILE")"

echo "Replaying channel message payload into flow '$FLOW_ID'"
echo "Payload: $PAYLOAD_FILE"
if [[ "$USE_SERVER" == "1" ]]; then
  echo "Target:  server=$SERVER (HTTP POST /runner/emit)"
else
  echo "Target:  local stub (no server)"
fi
echo

if [[ "$USE_SERVER" == "1" ]]; then
  "$BIN" runner emit \
    --flow "$FLOW_ID" \
    --tenant "$TENANT" \
    --user "$USER" \
    --payload "$PAYLOAD_JSON" \
    --server "$SERVER"
else
  "$BIN" runner emit \
    --flow "$FLOW_ID" \
    --tenant "$TENANT" \
    --user "$USER" \
    --payload "$PAYLOAD_JSON"
fi
