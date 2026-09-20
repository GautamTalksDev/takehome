#!/usr/bin/env bash
# Refresh crates/takehome-core/vendor/rules from data/rules (JSON only; no seed).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/data/rules"
DST="$ROOT/crates/takehome-core/vendor/rules"
rm -rf "$DST"
mkdir -p "$DST"
rsync -a --include='*/' --include='*.json' --exclude='*' "$SRC/" "$DST/"
if [ -f "$DST/signing.seed" ]; then
  echo "vendor must not contain signing.seed" >&2
  exit 1
fi
echo "vendored $(find "$DST" -name '*.json' | wc -l) json files"
