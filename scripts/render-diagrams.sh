#!/usr/bin/env bash
# Render docs/diagrams/*.mmd to web/site/public/diagrams/*.svg at build time.
# Uses pinned @mermaid-js/mermaid-cli. No runtime Mermaid JavaScript on the site.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/docs/diagrams"
OUT="$ROOT/web/site/public/diagrams"
mkdir -p "$OUT"

if ! command -v npx >/dev/null 2>&1; then
  echo "render-diagrams: npx required" >&2
  exit 1
fi

# Pin mermaid-cli so SVG output does not drift silently across CI images.
# GitHub-hosted runners (Ubuntu 24.04+) disallow unprivileged user namespaces;
# Chromium needs --no-sandbox there (same pattern as web/site/tests/lighthouse.mjs).
MMDC=(npx --yes @mermaid-js/mermaid-cli@11.4.2)
PUPPETEER_CFG="$ROOT/scripts/mermaid-puppeteer.json"

for mmd in "$SRC"/*.mmd; do
  base="$(basename "$mmd" .mmd)"
  svg="$OUT/${base}.svg"
  echo "render $base" >&2
  "${MMDC[@]}" -p "$PUPPETEER_CFG" -i "$mmd" -o "$svg" -b transparent
done

node "$ROOT/scripts/check-diagram-captions.mjs"
echo "diagrams rendered to $OUT"
