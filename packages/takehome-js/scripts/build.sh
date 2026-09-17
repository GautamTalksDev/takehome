#!/usr/bin/env bash
# Build wasm32-unknown-unknown takehome-wasm and bindgen into packages/takehome-js/wasm.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PKG="$ROOT/packages/takehome-js"
OUT="$PKG/wasm"
TARGET="${WASM_TARGET:-wasm32-unknown-unknown}"
# Keep in lockstep with workspace.dependencies.wasm-bindgen.
BINDGEN_VERSION="${WASM_BINDGEN_VERSION:-0.2.128}"
PROFILE="${WASM_PROFILE:-wasm}"

cd "$ROOT"

rustup target add "$TARGET" >/dev/null

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "installing wasm-bindgen-cli ${BINDGEN_VERSION}" >&2
  cargo install wasm-bindgen-cli --version "$BINDGEN_VERSION"
else
  installed="$(wasm-bindgen --version | awk '{print $2}')"
  if [ "$installed" != "$BINDGEN_VERSION" ]; then
    echo "wasm-bindgen ${installed} != ${BINDGEN_VERSION}; installing pinned CLI" >&2
    cargo install wasm-bindgen-cli --version "$BINDGEN_VERSION" --force
  fi
fi

cargo build -p takehome-wasm --target "$TARGET" --profile "$PROFILE"

WASM_IN="$ROOT/target/${TARGET}/${PROFILE}/takehome_wasm.wasm"
if [ ! -f "$WASM_IN" ]; then
  echo "missing $WASM_IN" >&2
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

wasm-bindgen "$WASM_IN" \
  --out-dir "$OUT" \
  --target web \
  --out-name takehome_wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Os --enable-bulk-memory -o "$OUT/takehome_wasm_bg.wasm" "$OUT/takehome_wasm_bg.wasm"
fi

echo "wrote $OUT/takehome_wasm_bg.wasm ($(wc -c < "$OUT/takehome_wasm_bg.wasm") bytes)"
