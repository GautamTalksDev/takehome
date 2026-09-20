#!/usr/bin/env bash
# A08:2025 item 47 — the built WASM must match the committed release hash.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM="$ROOT/packages/takehome-js/wasm/takehome_wasm_bg.wasm"
CLAIM="$ROOT/packages/takehome-js/wasm.sha256"

if [ ! -f "$CLAIM" ]; then
  echo "missing $CLAIM" >&2
  exit 1
fi
if [ ! -f "$WASM" ]; then
  echo "missing $WASM; run packages/takehome-js/scripts/build.sh first" >&2
  exit 1
fi

want="$(awk '{print $1}' "$CLAIM")"
got="$(sha256sum "$WASM" | awk '{print $1}')"
if [ "$got" != "$want" ]; then
  echo "WASM hash $got does not match release claim $want in $CLAIM" >&2
  exit 1
fi
echo "wasm hash ok $got"
