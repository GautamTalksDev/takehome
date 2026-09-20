#!/usr/bin/env bash
# House-style documentation linter. Failures are not suggestions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
exec node "$ROOT/scripts/docs-lint.mjs" "$@"
