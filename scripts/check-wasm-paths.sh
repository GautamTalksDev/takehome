#!/usr/bin/env bash
# Fail if the built WASM still embeds builder home paths (A08 / debuginfo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WASM="$ROOT/packages/takehome-js/wasm/takehome_wasm_bg.wasm"
if [ ! -f "$WASM" ]; then
  echo "missing $WASM; run packages/takehome-js/scripts/build.sh first" >&2
  exit 1
fi
if grep -a -F -e 'gautamtalksdev' -e '/home/' -e '/.cargo/' "$WASM"; then
  echo "WASM embeds a builder path (gautamtalksdev, /home/, or /.cargo/)" >&2
  exit 1
fi
echo "wasm path-ban: clean"
