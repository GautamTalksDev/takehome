#!/usr/bin/env bash
# Build wasm32-unknown-unknown takehome-wasm and bindgen into packages/takehome-js/wasm.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PKG="$ROOT/packages/takehome-js"
OUT="$PKG/wasm"
TARGET="${WASM_TARGET:-wasm32-unknown-unknown}"
# Keep in lockstep with workspace.dependencies.wasm-bindgen.
BINDGEN_VERSION="${WASM_BINDGEN_VERSION:-0.2.128}"
PROFILE="${WASM_PROFILE:-wasm}"

cd "$ROOT"

# DWARF and rustc metadata otherwise embed the builder's home directory.
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
REMAP=(
  --remap-path-prefix "${ROOT}=."
  --remap-path-prefix "${CARGO_HOME}=.cargo"
)
if [ -d "$RUSTUP_HOME" ]; then
  REMAP+=(--remap-path-prefix "${RUSTUP_HOME}=.rustup")
fi
export RUSTFLAGS="${RUSTFLAGS:-} ${REMAP[*]}"

rustup target add "$TARGET" >/dev/null

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "installing wasm-bindgen-cli ${BINDGEN_VERSION}" >&2
  cargo install wasm-bindgen-cli --version "$BINDGEN_VERSION"
else
  installed="$(wasm-bindgen --version | awk '{print $2}')"
  if [ "$installed" != "$BINDGEN_VERSION" ]; then
    echo "wasm-bindgen ${installed} != ${BINDGEN_VERSION}; installing pinned CLI" >&2
    cargo install wasm-bindgen-cli --version "$BINDGEN_VERSION" --force
  fi
fi

cargo build -p takehome-wasm --target "$TARGET" --profile "$PROFILE"

WASM_IN="$ROOT/target/${TARGET}/${PROFILE}/takehome_wasm.wasm"
if [ ! -f "$WASM_IN" ]; then
  echo "missing $WASM_IN" >&2
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

wasm-bindgen "$WASM_IN" \
  --out-dir "$OUT" \
  --target web \
  --out-name takehome_wasm

# Size pass is mandatory. Determinism comes from pinning Binaryen, not from
# skipping wasm-opt (that is how gzipped WASM grew 141,104 → 186,011).
PIN="$(tr -d '[:space:]' < "$PKG/binaryen-version")"
WASM_OPT="${HOME}/.local/binaryen-version_${PIN}/bin/wasm-opt"
if [ ! -x "$WASM_OPT" ]; then
  bash "$ROOT/scripts/install-binaryen.sh"
fi
bash "$ROOT/scripts/check-binaryen-version.sh" "$WASM_OPT"
"$WASM_OPT" -Oz --enable-bulk-memory --enable-bulk-memory-opt \
  -o "$OUT/takehome_wasm_bg.wasm" "$OUT/takehome_wasm_bg.wasm"

echo "wrote $OUT/takehome_wasm_bg.wasm ($(wc -c < "$OUT/takehome_wasm_bg.wasm") bytes)"

# Publish surface (A10 item 56): dist/ is the JS allowlist; the wasm
# binary stays at wasm/takehome_wasm_bg.wasm. Bindgen glue is copied
# into dist/ so consumers never receive wasm/*.js or source maps.
DIST="$PKG/dist"
rm -rf "$DIST"
mkdir -p "$DIST"
cp "$PKG/src/node.js" "$PKG/src/browser.js" "$PKG/src/index.js" "$PKG/src/index.d.ts" "$DIST/"
cp "$OUT/takehome_wasm.js" "$DIST/"
for dts in "$OUT"/*.d.ts; do
  if [ -f "$dts" ]; then
    cp "$dts" "$DIST/"
  fi
done
node --input-type=module -e '
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
const dist = process.argv[1];
for (const name of ["node.js", "browser.js"]) {
  const file = path.join(dist, name);
  let text = readFileSync(file, "utf8");
  text = text.replaceAll("../wasm/takehome_wasm.js", "./takehome_wasm.js");
  if (name === "browser.js") {
    text = text.replace(
      "await initWasm(source);",
      "await initWasm(source ?? new URL(\"../wasm/takehome_wasm_bg.wasm\", import.meta.url));",
    );
  }
  writeFileSync(file, text);
}
' "$DIST"
echo "wrote $DIST"
