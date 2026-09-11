#!/usr/bin/env bash
# Fail if IEEE-754 binary float types appear in netpay-core sources.
# compile_fail fixtures are excluded (they intentionally mention f32/f64).
# Doc/line comments are excluded so historical bug names in prose do not trip CI;
# executable code and string literals are still scanned.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

matches="$(
  rg -n --glob '*.rs' --glob '!**/compile_fail/**' '\bf(32|64)\b' crates/netpay-core \
    | grep -Ev ':[0-9]+:[[:space:]]*//' \
    | grep -Ev ':[0-9]+:[[:space:]]*/\*' \
    || true
)"

if [[ -n "${matches}" ]]; then
  echo "float-ban: forbidden IEEE-754 type token found:" >&2
  echo "${matches}" >&2
  exit 1
fi

echo "float-ban: clean"
