#!/usr/bin/env bash
# Install the pinned Binaryen release (wasm-opt) into $HOME/.local.
# Does not use distro packages — those float.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PIN_FILE="$ROOT/packages/takehome-js/binaryen-version"
PIN="$(tr -d '[:space:]' < "$PIN_FILE")"
if [ -n "${BINARYEN_VERSION:-}" ] && [ "$BINARYEN_VERSION" != "$PIN" ]; then
  echo "env BINARYEN_VERSION=${BINARYEN_VERSION} != pin ${PIN} in $PIN_FILE" >&2
  exit 1
fi

PREFIX="${HOME}/.local/binaryen-version_${PIN}"
if [ -x "$PREFIX/bin/wasm-opt" ]; then
  echo "binaryen ${PIN} already at $PREFIX" >&2
  exit 0
fi

os="$(uname -s)"
arch="$(uname -m)"
case "${os}-${arch}" in
  Linux-x86_64) triple="x86_64-linux" ;;
  Linux-aarch64) triple="aarch64-linux" ;;
  Darwin-arm64) triple="arm64-macos" ;;
  Darwin-x86_64) triple="x86_64-macos" ;;
  *)
    echo "no Binaryen ${PIN} tarball for ${os}-${arch}" >&2
    exit 1
    ;;
esac

asset="binaryen-version_${PIN}-${triple}.tar.gz"
url="https://github.com/WebAssembly/binaryen/releases/download/version_${PIN}/${asset}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
echo "downloading $url" >&2
curl -fsSL "$url" -o "$tmpdir/$asset"
tar -xzf "$tmpdir/$asset" -C "$tmpdir"
extracted="$tmpdir/binaryen-version_${PIN}"
if [ ! -x "$extracted/bin/wasm-opt" ]; then
  echo "tarball did not contain bin/wasm-opt" >&2
  exit 1
fi
mkdir -p "$(dirname "$PREFIX")"
rm -rf "$PREFIX"
mv "$extracted" "$PREFIX"
echo "installed binaryen ${PIN} to $PREFIX" >&2
