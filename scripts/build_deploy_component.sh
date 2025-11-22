#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

# Ensure the WASI target is available (ignore if already installed).
rustup target add wasm32-wasip2 >/dev/null 2>&1 || true

echo "Building deploy-plan component (wasm32-wasip2)..."
CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo build \
  --manifest-path "$ROOT/crates/deploy-plan-component/Cargo.toml" \
  --target wasm32-wasip2 \
  --release

ARTIFACT="$CARGO_TARGET_DIR/wasm32-wasip2/release/deploy_plan_component.wasm"
DEST="$ROOT/packs/deploy-generic/components/deploy_plan_component.wasm"

mkdir -p "$(dirname "$DEST")"
cp "$ARTIFACT" "$DEST"
echo "Copied $ARTIFACT -> $DEST"
