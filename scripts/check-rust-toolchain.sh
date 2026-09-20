#!/usr/bin/env bash
# Fail if rustc/cargo/clippy are not the channel pinned in rust-toolchain.toml.
# Same durability idea as check-binaryen-version.sh: a silent toolchain upgrade
# must fail CI, not surprise local clippy.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PIN_FILE="$ROOT/rust-toolchain.toml"
if [ ! -f "$PIN_FILE" ]; then
  echo "missing $PIN_FILE" >&2
  exit 1
fi
PIN="$(
  awk -F'"' '
    /^channel[[:space:]]*=/ { print $2; exit }
  ' "$PIN_FILE"
)"
if [ -z "$PIN" ]; then
  echo "could not read channel from $PIN_FILE" >&2
  exit 1
fi

# Clippy's crate version is 0.1.N when rustc is 1.N.0 (e.g. 1.90.0 → 0.1.90).
CLIPPY_PIN="$(
  python3 - <<PY
pin = "$PIN"
parts = pin.split(".")
if len(parts) >= 2 and parts[0] == "1":
    print(f"0.1.{parts[1]}")
else:
    raise SystemExit(f"unexpected rust channel {pin!r}")
PY
)"

assert_contains() {
  local tool="$1"
  local needle="$2"
  local out
  out="$("$tool" --version 2>&1 || true)"
  if ! grep -Fqw "$needle" <<<"$out"; then
    echo "$tool is not Rust pin ${needle}: $out" >&2
    echo "expected channel ${PIN} from $PIN_FILE" >&2
    exit 1
  fi
  echo "$tool ok ${needle} ($out)"
}

assert_contains rustc "$PIN"
assert_contains cargo "$PIN"

if command -v clippy-driver >/dev/null 2>&1; then
  assert_contains clippy-driver "$CLIPPY_PIN"
else
  out="$(cargo clippy -V 2>&1 || true)"
  if ! grep -Fqw "$CLIPPY_PIN" <<<"$out"; then
    echo "cargo clippy is not pin ${CLIPPY_PIN}: $out" >&2
    exit 1
  fi
  echo "clippy ok ${CLIPPY_PIN} ($out)"
fi

# Active toolchain must be the pin (not a newer default from the runner image).
active="$(rustup show active-toolchain 2>/dev/null || true)"
if [ -n "$active" ] && ! grep -Fq "$PIN" <<<"$active"; then
  echo "active toolchain is not ${PIN}: $active" >&2
  exit 1
fi
echo "rust-toolchain ok ${PIN}"
