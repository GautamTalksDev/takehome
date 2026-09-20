#!/usr/bin/env bash
# A05:2025 — D1 queries must be prepared statements with bound parameters.
# Fails on string-concatenated or interpolated SQL in services/api/src.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
SRC="services/api/src"
FOUND=0

if grep -RInE --include='*.js' \
  '(SELECT|INSERT|UPDATE|DELETE|WITH)[[:space:]].*\$\{' \
  "$SRC"; then
  echo "SQL-BAN: interpolated SQL (\${) is forbidden. Use prepare() + bind()." >&2
  FOUND=1
fi

if grep -RInE --include='*.js' \
  "['\"\`](SELECT|INSERT|UPDATE|DELETE|WITH)[^'\`\"]*['\"\`]\s*\+" \
  "$SRC"; then
  echo "SQL-BAN: concatenated SQL strings are forbidden. Use prepare() + bind()." >&2
  FOUND=1
fi

if grep -RInE --include='*.js' '\.prepare\([A-Za-z_$]' "$SRC"; then
  echo "SQL-BAN: prepare() must take a SQL string literal, not a variable." >&2
  FOUND=1
fi

if grep -RInE --include='*.js' '\.exec\s*\(' "$SRC"; then
  echo "SQL-BAN: db.exec() is forbidden. Use prepare() + bind()." >&2
  FOUND=1
fi

if [ "$FOUND" -eq 1 ]; then
  echo "SQL-BAN VIOLATION: string-concatenated or unprepared SQL in $SRC." >&2
  exit 1
fi
echo "sql-ban: clean"
