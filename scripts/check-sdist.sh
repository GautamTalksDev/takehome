#!/usr/bin/env bash
# Prove the Python sdist rebuilds from source with vendored rules (not the wheel).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import filecmp
import pathlib
src = pathlib.Path("data/rules")
dst = pathlib.Path("crates/takehome-core/vendor/rules")
src_files = sorted(p.relative_to(src) for p in src.rglob("*.json"))
dst_files = sorted(p.relative_to(dst) for p in dst.rglob("*.json"))
missing = set(src_files) - set(dst_files)
extra = set(dst_files) - set(src_files)
if missing or extra:
    raise SystemExit(f"vendor drift missing={missing} extra={extra}")
for rel in src_files:
    if not filecmp.cmp(src / rel, dst / rel, shallow=False):
        raise SystemExit(f"byte mismatch {rel}")
seed = dst / "signing.seed"
if seed.exists():
    raise SystemExit("signing.seed must not be vendored")
print(f"vendor json identical: {len(src_files)} files")
PY

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

# Isolated from the repo: only the sdist tarball and M1 vectors travel.
VECTORS="$WORKDIR/pdoc_ontario_2026_01.json"
cp crates/takehome-core/tests/vectors/pdoc_ontario_2026_01.json "$VECTORS"

TOOLS_VENV="$WORKDIR/tools"
python3 -m venv "$TOOLS_VENV"
"$TOOLS_VENV/bin/pip" install -q maturin
(
  cd packages/takehome-py
  "$TOOLS_VENV/bin/maturin" sdist -o "$WORKDIR/sdist"
)
SDIST="$(find "$WORKDIR/sdist" -name 'takehome_ca-*.tar.gz' | head -n1)"
if [ -z "$SDIST" ]; then
  echo "maturin sdist produced no takehome_ca tarball" >&2
  exit 1
fi

tar -tzf "$SDIST" | grep -E 'vendor/rules/.+\.json$' > "$WORKDIR/sdist-json.txt"
if [ ! -s "$WORKDIR/sdist-json.txt" ]; then
  echo "sdist is missing vendored rule JSON" >&2
  tar -tzf "$SDIST" | head
  exit 1
fi
if tar -tzf "$SDIST" | grep -q 'signing.seed'; then
  echo "sdist must not contain signing.seed" >&2
  exit 1
fi
echo "sdist contains $(wc -l < "$WORKDIR/sdist-json.txt") vendored json paths"

BUILD_VENV="$WORKDIR/build"
python3 -m venv "$BUILD_VENV"
# Build deps may come from the local pip cache; do not look at the repo.
"$BUILD_VENV/bin/pip" install -q "$SDIST"
"$BUILD_VENV/bin/python" - <<PY
import json
from pathlib import Path
import takehome_ca

vectors = json.loads(Path("$VECTORS").read_text())["vectors"]
assert len(vectors) == 20
for vector in vectors:
    parsed = json.loads(takehome_ca.calculate(json.dumps(vector["request"])))
    assert parsed["employee"]["net_pay"] == vector["expected"]["net_pay"], vector["id"]
    assert parsed["employee"]["federal_tax"] == vector["expected"]["federal_tax"], vector["id"]
print("sdist M1 vectors: 20/20")
print("engine_build_sha", takehome_ca.engine_build_sha())
PY
echo "sdist rebuild: ok"
