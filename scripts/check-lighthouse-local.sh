#!/usr/bin/env bash
# Strict Lighthouse gate for local / pre-push / release: performance >= 0.99.
# Shared GHA runners use median-of-three against 0.95 instead (see lighthouse.mjs).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/web/site"
if [ ! -d dist ]; then
  npm run build
fi
export TAKEHOME_LIGHTHOUSE_MODE=local
npm run test:lighthouse:unit
npm run test:lighthouse:local
