#!/usr/bin/env bash
# Fails if the retired product name appears outside historical notes.
#
# Permitted hits are records of work done under the old name:
# CHANGELOG.md, CONFORMANCE.md, docs/ADR-*.md, docs/findings/*.md,
# the dated operator log in docs/FIVE-MINUTE-TEST.md, and the pre-rename
# engine identity fixture.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

allowlisted() {
  local f="$1"
  case "$f" in
    CHANGELOG.md) return 0 ;;
    CONFORMANCE.md) return 0 ;;
    docs/ADR-*.md) return 0 ;;
    docs/ADR-001-decimal.md|docs/ADR-002-published-constants.md|docs/ADR-003-rounding-compat.md) return 0 ;;
    docs/findings/*.md) return 0 ;;
    docs/findings/001-k2-maximum-in-reaching-period.md|docs/findings/002-alberta-half-cent-float.md|docs/findings/002-pdoc-midpoint-direction.md) return 0 ;;
    docs/FIVE-MINUTE-TEST.md) return 0 ;;
    scripts/name-ban.sh) return 0 ;;
    packages/netpay-js/tests/fixtures/pre-rename-engine.json) return 0 ;;
    packages/takehome-js/tests/fixtures/pre-rename-engine.json) return 0 ;;
  esac
  return 1
}

FOUND=0
while IFS= read -r line; do
  file="${line%%:*}"
  file="${file#./}"
  if allowlisted "$file"; then
    continue
  fi
  printf '%s\n' "$line"
  FOUND=1
done < <(
  grep -rIn -i \
    --exclude-dir=.git \
    --exclude-dir=target \
    --exclude-dir=node_modules \
    --exclude-dir=.venv \
    --exclude-dir=dist \
    --exclude-dir=.astro \
    --exclude-dir=.wrangler \
    --exclude-dir=playwright-report \
    --exclude-dir=test-results \
    --exclude-dir=blob-report \
    --exclude-dir=pdoc-cache \
    --exclude-dir=pdoc-screenshots \
    --exclude-dir=grids \
    --exclude-dir=wasm \
    --exclude='*.wasm' \
    --exclude='lighthouse-report.json' \
    -e 'netpay' .
)

if [ "$FOUND" -eq 1 ]; then
  echo "NAME BAN VIOLATION: leftover Netpay product name outside historical notes." >&2
  exit 1
fi
echo "name-ban: clean"
