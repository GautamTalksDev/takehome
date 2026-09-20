#!/usr/bin/env bash
# Record SHA-256 of packages/takehome-js/wasm/takehome_wasm_bg.wasm.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM="$ROOT/packages/takehome-js/wasm/takehome_wasm_bg.wasm"
OUT="$ROOT/packages/takehome-js/wasm.sha256"
if [ ! -f "$WASM" ]; then
  echo "missing $WASM; run packages/takehome-js/scripts/build.sh first" >&2
  exit 1
fi
(cd "$(dirname "$WASM")" && sha256sum "$(basename "$WASM")") > "$OUT"
cat "$OUT"
