#!/usr/bin/env bash
# Fails if secret-bearing or generated paths are tracked in git.
# Durable fix for the 978d611 .so/.pyc incident: a check, not care.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

FOUND=0
while IFS= read -r f; do
  case "$f" in
    .env|.env.*|.dev.vars|.npmrc|*/.env|*/.env.*|*/.dev.vars|*/.npmrc)
      if [[ "$f" == *.env.example || "$f" == */.env.example ]]; then
        continue
      fi
      ;;
    .wrangler/*|*/.wrangler/*) ;;
    node_modules/*|*/node_modules/*) ;;
    target/*|*/target/*) ;;
    dist/*|*/dist/*) ;;
    __pycache__/*|*/__pycache__/*) ;;
    *.pyc|*.so|*.whl|*.key|*.pem|*.p12) ;;
    data/pdoc-screenshots/*)
      if [[ "$f" == data/pdoc-screenshots/.gitkeep ]]; then
        continue
      fi
      ;;
    data/pdoc-cache/records|data/pdoc-cache/records/*)
      ;;
    *.tar.zst|*.tar.zst.*)
      ;;
    data/pdoc-cache/*.log|data/pdoc-cache/*checkpoint.json|data/pdoc-cache/last-run-conditions.json)
      ;;
    *) continue ;;
  esac
  printf '%s\n' "$f"
  FOUND=1
done < <(git ls-files)

if [ "$FOUND" -eq 1 ]; then
  echo "TRACKED-BAN VIOLATION: secret or generated path is tracked. Remove it from HEAD (and from history if it was a secret)." >&2
  exit 1
fi
echo "tracked-ban: clean"
