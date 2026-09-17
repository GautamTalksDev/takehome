#!/usr/bin/env bash
# Fails if IEEE-754 types appear anywhere in the money path.
set -euo pipefail
PATHS=("crates/takehome-core/src" "tools/ingest/src" "tools/ingest/tests")
PATTERN='\b(f32|f64)\b|\bas f(32|64)\b|to_f64|to_f32|from_f64|from_f32|parse::<f(32|64)>'
FOUND=0
for p in "${PATHS[@]}"; do
  if grep -rInE --include='*.rs' "$PATTERN" "$p"; then
    FOUND=1
  fi
done
if [ "$FOUND" -eq 1 ]; then
  echo "FLOAT BAN VIOLATION: binary floating point found in a money path." >&2
  exit 1
fi
echo "float-ban: clean"
