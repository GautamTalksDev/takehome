#!/usr/bin/env bash
# Native CLI + WASM + node:test identity suite (spec §5.4).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"

cargo build -p takehome-cli --release
export TAKEHOME_CLI="$ROOT/target/release/takehome"

if [ ! -f "$ROOT/packages/takehome-js/tests/fixtures/grid-sample-500.json" ]; then
  cargo run -p takehome-grid-gen -- --identity-sample 500 --out "$ROOT/packages/takehome-js/tests/fixtures"
fi

bash "$ROOT/packages/takehome-js/scripts/build.sh"
bash "$ROOT/scripts/check-binaryen-version.sh"
bash "$ROOT/scripts/check-wasm-hash.sh"
bash "$ROOT/scripts/check-wasm-paths.sh"
cd "$ROOT/packages/takehome-js"
node --test tests/*.test.js
