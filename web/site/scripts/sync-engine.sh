#!/usr/bin/env bash
# Copy the WASM module and bindgen glue into public/engine/.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SITE="$(cd "$(dirname "$0")/.." && pwd)"
WASM_SRC="$ROOT/packages/takehome-js/wasm"
if [ ! -f "$WASM_SRC/takehome_wasm_bg.wasm" ]; then
  bash "$ROOT/packages/takehome-js/scripts/build.sh"
fi
mkdir -p "$SITE/public/engine"
cp "$WASM_SRC/takehome_wasm.js" "$SITE/public/engine/"
cp "$WASM_SRC/takehome_wasm_bg.wasm" "$SITE/public/engine/"
cp "$ROOT/data/plans.json" "$SITE/src/data/plans.json"
echo "synced engine wasm ($(wc -c < "$SITE/public/engine/takehome_wasm_bg.wasm") bytes)"
