#!/usr/bin/env bash
# Fail if wasm-opt is missing or is not the pinned Binaryen version.
# A silent toolchain upgrade moving the WASM hash is the failure this catches.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PIN_FILE="$ROOT/packages/takehome-js/binaryen-version"
if [ ! -f "$PIN_FILE" ]; then
  echo "missing $PIN_FILE" >&2
  exit 1
fi
PIN="$(tr -d '[:space:]' < "$PIN_FILE")"
if [ -n "${BINARYEN_VERSION:-}" ] && [ "$BINARYEN_VERSION" != "$PIN" ]; then
  echo "env BINARYEN_VERSION=${BINARYEN_VERSION} != pin ${PIN} in $PIN_FILE" >&2
  exit 1
fi

PREFIX="${HOME}/.local/binaryen-version_${PIN}"
WASM_OPT="${1:-$PREFIX/bin/wasm-opt}"
if [ ! -x "$WASM_OPT" ]; then
  echo "missing wasm-opt at $WASM_OPT; run scripts/install-binaryen.sh" >&2
  exit 1
fi

out="$("$WASM_OPT" --version 2>&1 || true)"
if ! grep -Fqw "$PIN" <<<"$out"; then
  echo "wasm-opt is not Binaryen ${PIN}: $out" >&2
  exit 1
fi
echo "binaryen ok ${PIN} ($WASM_OPT)"
